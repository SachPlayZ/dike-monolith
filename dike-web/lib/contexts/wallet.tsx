"use client";

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  type ReactNode,
} from "react";
import {
  kitConnect,
  kitDisconnect,
  kitSign,
  kitGetAddress,
  kitGetNetwork,
  initWalletKit,
} from "@/lib/stellar/wallet";
import { assertValidConfiguration, networkConfig } from "@/lib/stellar/config";
import { fetchWalletPermissions } from "@/lib/api/authz";
import type { WalletPermissions } from "@/lib/types";
import { feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";

interface WalletContextType {
  address: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  networkError: string | null;
  permissions: WalletPermissions | null;
  permissionsLoading: boolean;
  connect: () => Promise<void>;
  disconnect: () => void;
  sign: (xdr: string, options?: { sponsored?: boolean; method?: string }) => Promise<string>;
}

const WalletContext = createContext<WalletContextType | null>(null);

const ADDRESS_KEY = "dike:wallet:address";

export function WalletProvider({ children }: { children: ReactNode }) {
  const [address, setAddress] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [networkError, setNetworkError] = useState<string | null>(null);
  const [permissions, setPermissions] = useState<WalletPermissions | null>(null);

  // Restore persisted address and validate network on mount
  useEffect(() => {
    if (typeof window === "undefined") return;
    const stored = sessionStorage.getItem(ADDRESS_KEY);
    if (!stored) return;

    initWalletKit().then(() => {
      Promise.all([kitGetAddress(), kitGetNetwork()])
        .then(([addr, walletNetwork]) => {
          if (walletNetwork !== networkConfig.networkPassphrase) {
            setNetworkError(
              `Wrong wallet network. Switch to ${networkConfig.label}.`,
            );
            sessionStorage.removeItem(ADDRESS_KEY);
            return;
          }
          if (addr === stored) setAddress(addr);
          else sessionStorage.removeItem(ADDRESS_KEY);
        })
        .catch(() => sessionStorage.removeItem(ADDRESS_KEY));
    });
  }, []);

  const connect = useCallback(async () => {
    setIsConnecting(true);
    setNetworkError(null);
    try {
      assertValidConfiguration();
      // Open the wallet picker/auth modal first — a module must be
      // selected before getNetwork() can be called on it.
      const addr = await kitConnect();

      // Verify network now that a wallet module is selected
      let walletNet: string;
      try {
        walletNet = await kitGetNetwork();
      } catch {
        throw new Error("Could not determine wallet network. Connection blocked until the wallet reports its active network.");
      }
      if (walletNet !== networkConfig.networkPassphrase) {
        await kitDisconnect().catch(() => {});
        setNetworkError(
          `Wrong wallet network. Switch to ${networkConfig.label}.`
        );
        return;
      }

      setAddress(addr);
      sessionStorage.setItem(ADDRESS_KEY, addr);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.includes("closed the modal")) {
        setNetworkError(msg);
      }
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    kitDisconnect().catch(() => {});
    setAddress(null);
    setPermissions(null);
    sessionStorage.removeItem(ADDRESS_KEY);
  }, []);

  const sign = useCallback(
    async (xdr: string, options?: { sponsored?: boolean; method?: string }): Promise<string> => {
      if (!address) throw new Error("Wallet not connected");
      assertValidConfiguration();

      // Verify network before signing
      let walletNet: string;
      try {
        walletNet = await kitGetNetwork();
      } catch {
        throw new Error("Could not determine wallet network. Signing blocked until the wallet reports its active network.");
      }
      if (walletNet !== networkConfig.networkPassphrase) {
        throw new Error(
          `Wrong wallet network. Switch to ${networkConfig.label} before signing.`
        );
      }

      const sponsored = options?.sponsored === true;
      const maximumFee = sponsored ? "0 XLM (paid by Dike sponsor)" : formatFeeXlm(feeFromXdr(xdr));
      const methodLabel = options?.method ? `\nContract action: ${options.method}` : "";
      const approved = window.confirm(
        `Review transaction before opening your wallet\n\nNetwork: ${networkConfig.label}${methodLabel}\nYour wallet network fee: ${maximumFee}\n\nAsset transfers, bonds, and liquidity amounts still require your approval. Continue to wallet signing?`,
      );
      if (!approved) throw new Error("Transaction signing cancelled.");

      return kitSign(xdr);
    },
    [address]
  );

  useEffect(() => {
    if (!address) return;

    let cancelled = false;

    void fetchWalletPermissions(address)
      .then((nextPermissions) => {
        if (cancelled) return;
        setPermissions(nextPermissions);
      })
      .catch(() => {
        if (cancelled) return;
        setPermissions(null);
      });

    return () => {
      cancelled = true;
    };
  }, [address]);

  const permissionsLoading = Boolean(address) && permissions?.address !== address;

  return (
    <WalletContext.Provider
      value={{
        address,
        isConnected: Boolean(address),
        isConnecting,
        networkError,
        permissions,
        permissionsLoading,
        connect,
        disconnect,
        sign,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet(): WalletContextType {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error("useWallet must be used inside WalletProvider");
  return ctx;
}

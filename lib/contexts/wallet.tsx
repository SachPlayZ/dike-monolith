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
import { networkConfig } from "@/lib/stellar/config";
import { fetchWalletPermissions } from "@/lib/api/authz";
import type { WalletPermissions } from "@/lib/types";

interface WalletContextType {
  address: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  networkError: string | null;
  permissions: WalletPermissions | null;
  permissionsLoading: boolean;
  connect: () => Promise<void>;
  disconnect: () => void;
  sign: (xdr: string) => Promise<string>;
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
      kitGetAddress()
        .then((addr) => {
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
      // Verify network before connecting
      const walletNet = await kitGetNetwork().catch(() => null);
      if (walletNet && walletNet !== networkConfig.networkPassphrase) {
        setNetworkError(
          `Wrong network. Expected "${networkConfig.networkPassphrase}", wallet is on "${walletNet}".`
        );
        return;
      }

      const addr = await kitConnect();
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
    async (xdr: string): Promise<string> => {
      if (!address) throw new Error("Wallet not connected");

      // Verify network before signing
      const walletNet = await kitGetNetwork().catch(() => null);
      if (walletNet && walletNet !== networkConfig.networkPassphrase) {
        throw new Error(
          `Wrong network. Switch wallet to Stellar Testnet before signing.`
        );
      }

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

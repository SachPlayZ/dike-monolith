"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { useWallet } from "@/lib/contexts/wallet";
import { buildCloseTrading } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState } from "@/lib/types";

interface CloseTradingButtonProps {
  marketId: string;
  expiry: number;
}

// Contract-side close_trading is permissionless (no require_auth) — any wallet
// can call it once expiry passes. Gated to admins here purely as a UI choice
// to keep the action out of regular traders' way.
export function CloseTradingButton({ marketId, expiry }: CloseTradingButtonProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign, permissions } = useWallet();
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const isExpired = Date.now() / 1000 >= expiry;
  if (!isExpired || !permissions?.canAdmin) return null;

  async function handleClose() {
    if (!address) return;
    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildCloseTrading(address, marketId);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  return (
    <div className="rounded-2xl bg-orange-500/[0.06] border border-orange-500/[0.18] px-5 py-4 space-y-3">
      <div>
        <p className="text-xs font-semibold uppercase tracking-widest text-orange-300/80">
          Trading window expired
        </p>
        <p className="text-xs text-white/40 mt-1">
          Expiry passed but on-chain status is still Live — no one has closed trading yet.
        </p>
      </div>
      {isConnected ? (
        <button
          onClick={handleClose}
          disabled={isPending}
          className="px-4 py-2 rounded-lg bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-200 disabled:opacity-50"
        >
          {isPending ? "Closing…" : "Close Trading"}
        </button>
      ) : (
        <button
          onClick={connect}
          className="px-4 py-2 rounded-lg bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-200"
        >
          Connect Wallet
        </button>
      )}
      {simulatedFee && (
        <p className="text-[10px] text-white/30">Simulated network fee: {simulatedFee}</p>
      )}
      <TxStateDisplay state={txState} />
    </div>
  );
}

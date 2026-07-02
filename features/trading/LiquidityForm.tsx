"use client";

import { useState, useTransition, useEffect } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildAmmAddLiquidity,
  buildAmmRemoveLiquidity,
  buildAmmClaimLpFees,
  ammGetLpBalance,
  ammGetClaimableLpFees,
  ammGetPool,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState } from "@/lib/types";

type Mode = "add" | "remove";

interface LiquidityFormProps {
  poolId: string;
}

export function LiquidityForm({ poolId }: LiquidityFormProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign } = useWallet();
  const [expanded, setExpanded] = useState(false);
  const [mode, setMode] = useState<Mode>("add");
  const [amountInput, setAmountInput] = useState("");
  const [lpBalance, setLpBalance] = useState<string | null>(null);
  const [claimableFees, setClaimableFees] = useState<string | null>(null);
  const [lpSupply, setLpSupply] = useState<string | null>(null);
  const [yesReserve, setYesReserve] = useState<string | null>(null);
  const [noReserve, setNoReserve] = useState<string | null>(null);
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isClaiming, startClaimTransition] = useTransition();
  const [isPending, startTransition] = useTransition();

  function refreshLpState() {
    if (!address) return;
    ammGetLpBalance(address, address, poolId)
      .then((bal) => setLpBalance(bal))
      .catch(() => setLpBalance(null));
    ammGetClaimableLpFees(address, address, poolId)
      .then((fees) => setClaimableFees(fees))
      .catch(() => setClaimableFees(null));
    ammGetPool(address, poolId)
      .then((pool) => {
        setLpSupply(pool.lpSupply);
        setYesReserve(pool.yesReserve);
        setNoReserve(pool.noReserve);
      })
      .catch(() => {
        setLpSupply(null);
        setYesReserve(null);
        setNoReserve(null);
      });
  }

  let expectedLpShares: bigint | null = null;
  if (
    mode === "add" &&
    amountInput &&
    lpSupply !== null &&
    yesReserve !== null &&
    noReserve !== null
  ) {
    try {
      const amount = parseUsdc(amountInput);
      const totalShares = BigInt(lpSupply);
      const yes = BigInt(yesReserve);
      const no = BigInt(noReserve);
      if (amount > 0n && totalShares > 0n && yes > 0n && no > 0n) {
        const yesShares = (totalShares * amount) / yes;
        const noShares = (totalShares * amount) / no;
        expectedLpShares = yesShares < noShares ? yesShares : noShares;
      }
    } catch {
      expectedLpShares = null;
    }
  }

  useEffect(() => {
    if (!address || !expanded) return;
    refreshLpState();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [address, expanded, poolId]);

  async function handleClaimFees() {
    if (!address) return;
    startClaimTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildAmmClaimLpFees(address, poolId);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        refreshLpState();
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  async function handleSubmit() {
    if (!address || !amountInput) return;
    startTransition(async () => {
      try {
        const rawAmount = parseUsdc(amountInput).toString();
        setTxState({ status: "building", hash: null, error: null });
        const xdr =
          mode === "add"
            ? await buildAmmAddLiquidity(address, poolId, rawAmount)
            : await buildAmmRemoveLiquidity(address, poolId, rawAmount);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        refreshLpState();
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] px-5 py-4 text-center space-y-3">
        <p className="text-xs text-white/40">Connect wallet to provide liquidity</p>
        <button
          onClick={connect}
          className="px-4 py-2 rounded-full bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-300"
        >
          Connect Wallet
        </button>
      </div>
    );
  }

  return (
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
      {/* Collapsible header */}
      <button
        onClick={() => setExpanded((v) => !v)}
        className="w-full px-5 py-4 flex items-center justify-between border-b border-white/[0.05] hover:bg-white/[0.02] transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">Liquidity</p>
          {lpBalance !== null && (
            <p className="text-xs text-white/40 mt-0.5 font-mono">
              {formatUsdc(BigInt(lpBalance))} LP
            </p>
          )}
        </div>
        <span className={cn(
          "text-white/30 text-sm transition-transform duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]",
          expanded && "rotate-180"
        )}>
          ↓
        </span>
      </button>

      {expanded && (
        <div className="p-5 space-y-4 animate-in fade-in-0 slide-in-from-top-2 duration-200">
          {/* Add / Remove tab */}
          <div className="grid grid-cols-2 rounded-xl bg-white/[0.05] p-1 gap-0.5">
            {(["add", "remove"] as Mode[]).map((m) => (
              <button
                key={m}
                onClick={() => { setMode(m); setAmountInput(""); }}
                className={cn(
                  "py-2 rounded-lg text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]",
                  mode === m
                    ? "bg-white/[0.10] text-white"
                    : "text-white/30 hover:text-white/60"
                )}
              >
                {m}
              </button>
            ))}
          </div>

          {/* LP balance chip (if removing) */}
          {mode === "remove" && lpBalance !== null && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-white/[0.04] border border-white/[0.07]">
              <span className="text-xs text-white/40">LP balance</span>
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold font-mono text-white/70">
                  {formatUsdc(BigInt(lpBalance))}
                </span>
                <button
                  onClick={() => setAmountInput(formatUsdc(BigInt(lpBalance)))}
                  className="px-2 py-0.5 rounded-md bg-white/[0.08] text-[10px] font-bold uppercase tracking-widest text-white/50 hover:text-white/80 hover:bg-white/[0.12] transition-all duration-200"
                >
                  Max
                </button>
              </div>
            </div>
          )}

          {/* Claimable LP fees */}
          {claimableFees !== null && BigInt(claimableFees) > 0n && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-orange-500/[0.06] border border-orange-500/[0.15]">
              <div>
                <span className="text-xs text-white/40">Claimable fees</span>
                <p className="text-sm font-semibold font-mono text-orange-300">
                  {formatUsdc(BigInt(claimableFees))}
                </p>
              </div>
              <button
                onClick={handleClaimFees}
                disabled={isClaiming || isPending}
                className="px-3 py-1.5 rounded-lg bg-orange-500/15 border border-orange-500/25 text-orange-300 text-[10px] font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-200 disabled:opacity-50"
              >
                {isClaiming ? "Claiming…" : "Claim"}
              </button>
            </div>
          )}

          {/* Amount input */}
          <div className={cn(
            "rounded-xl border px-4 py-3.5 transition-all duration-200",
            "bg-white/[0.04] border-white/[0.08] focus-within:border-white/[0.16] focus-within:bg-white/[0.06]"
          )}>
            <p className="text-[10px] uppercase tracking-[0.12em] text-white/30 mb-2">
              {mode === "add" ? "USDC Amount" : "LP Shares"}
            </p>
            <div className="flex items-baseline gap-2">
              <input
                type="number"
                min="0"
                step="0.01"
                placeholder="0"
                value={amountInput}
                onChange={(e) => setAmountInput(e.target.value)}
                className="flex-1 bg-transparent text-2xl font-semibold text-white placeholder-white/15 outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none w-full min-w-0"
              />
              <span className="text-sm text-white/30 shrink-0">
                {mode === "add" ? "USDC" : "LP"}
              </span>
            </div>
          </div>

          {mode === "add" && expectedLpShares !== null && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-white/[0.04] border border-white/[0.07]">
              <span className="text-xs text-white/40">Expected LP shares</span>
              <span className="text-sm font-semibold font-mono text-white/70">
                {formatUsdc(expectedLpShares)}
              </span>
            </div>
          )}

          {simulatedFee && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-white/[0.04] border border-white/[0.07]">
              <span className="text-xs text-white/40">Simulated network fee</span>
              <span className="text-sm font-semibold font-mono text-white/70">
                {simulatedFee}
              </span>
            </div>
          )}

          {/* CTA */}
          <button
            onClick={handleSubmit}
            disabled={isPending || isClaiming || !amountInput}
            className={cn(
              "w-full py-3.5 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)] active:scale-[0.98]",
              amountInput && !isPending
                ? "bg-white/[0.08] border border-white/[0.14] text-white/70 hover:bg-white/[0.12] hover:text-white"
                : "bg-white/[0.04] border border-white/[0.07] text-white/25 cursor-not-allowed"
            )}
          >
            {isPending
              ? "Processing…"
              : mode === "add"
              ? "Add Liquidity"
              : "Remove Liquidity"}
          </button>

          <TxStateDisplay state={txState} />
        </div>
      )}
    </div>
  );
}

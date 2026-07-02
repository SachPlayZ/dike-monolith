"use client";

import { useState, useTransition, useEffect } from "react";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/contexts/wallet";
import {
  ammQuoteBuyYes,
  ammQuoteBuyNo,
  ammQuoteSellYes,
  ammQuoteSellNo,
  buildAmmBuyYes,
  buildAmmBuyNo,
  buildAmmSellYes,
  buildAmmSellNo,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState } from "@/lib/types";

type Side = "buy" | "sell";
type Token = "yes" | "no";

interface TradeFormProps {
  marketId: string;
  poolId: string;
  marketQuestion: string;
}

const DEFAULT_SLIPPAGE_BPS = 50;
const TRADE_DEADLINE_SECS = 300;

export function TradeForm({ marketId, poolId, marketQuestion }: TradeFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [side, setSide] = useState<Side>("buy");
  const [token, setToken] = useState<Token>("yes");
  const [amountInput, setAmountInput] = useState("");
  const [quote, setQuote] = useState<{ amountOut: string; feeBps: number; priceImpactBps: number } | null>(null);
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  useEffect(() => {
    if (!address || !amountInput || parseFloat(amountInput) <= 0) {
      setQuote(null);
      setSimulatedFee(null);
      return;
    }
    setQuoting(true);
    const timer = setTimeout(async () => {
      try {
        const rawIn = parseUsdc(amountInput).toString();
        let q;
        if (side === "buy") {
          q = token === "yes"
            ? await ammQuoteBuyYes(address, poolId, rawIn)
            : await ammQuoteBuyNo(address, poolId, rawIn);
        } else {
          q = token === "yes"
            ? await ammQuoteSellYes(address, poolId, rawIn)
            : await ammQuoteSellNo(address, poolId, rawIn);
        }
        setQuote({ amountOut: q.amountOut, feeBps: q.feeBps, priceImpactBps: q.priceImpactBps });
      } catch (e) {
        setQuote(null);
        setSimulatedFee(null);
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      } finally {
        setQuoting(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [address, amountInput, side, token, poolId]);

  async function handleTrade() {
    if (!address || !amountInput || !quote) return;
    startTransition(async () => {
      try {
        const rawIn = parseUsdc(amountInput).toString();
        const minOut = applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS).toString();
        setTxState({ status: "building", hash: null, error: null });
        let xdr: string;
        if (side === "buy") {
          xdr = token === "yes"
            ? await buildAmmBuyYes(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS)
            : await buildAmmBuyNo(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS);
        } else {
          xdr = token === "yes"
            ? await buildAmmSellYes(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS)
            : await buildAmmSellNo(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS);
        }
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        setQuote(null);
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] px-5 py-6 text-center space-y-3">
        <p className="text-sm text-white/40">Connect wallet to trade</p>
        <button
          onClick={connect}
          className="px-5 py-2.5 rounded-full bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-300 active:scale-[0.98]"
        >
          Connect Wallet
        </button>
      </div>
    );
  }

  const isExecuting = isPending || quoting;
  const canTrade = Boolean(quote && amountInput && !isExecuting);

  return (
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
      {/* Header */}
      <div className="px-5 pt-5 pb-4 border-b border-white/[0.05]">
        <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30 mb-1.5">Trade</p>
        <p className="text-xs text-white/40 line-clamp-2 leading-relaxed font-heading">{marketQuestion}</p>
        <p className="text-[10px] font-mono text-white/20 mt-1">#{marketId.slice(0, 8)}…</p>
      </div>

      <div className="p-5 space-y-4">
        {/* Buy / Sell tab */}
        <div className="grid grid-cols-2 rounded-xl bg-white/[0.05] p-1 gap-0.5">
          {(["buy", "sell"] as Side[]).map((s) => (
            <button
              key={s}
              onClick={() => { setSide(s); setQuote(null); }}
              className={cn(
                "py-2 rounded-lg text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]",
                side === s
                  ? "bg-white/[0.10] text-white"
                  : "text-white/30 hover:text-white/60"
              )}
            >
              {s}
            </button>
          ))}
        </div>

        {/* YES / NO selector */}
        <div className="grid grid-cols-2 gap-2">
          {(["yes", "no"] as Token[]).map((t) => (
            <button
              key={t}
              onClick={() => { setToken(t); setQuote(null); }}
              className={cn(
                "py-3 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)] border",
                t === "yes"
                  ? token === "yes"
                    ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-400 shadow-[0_0_24px_rgba(52,211,153,0.08)]"
                    : "bg-transparent border-white/[0.06] text-white/30 hover:text-emerald-400/60 hover:border-emerald-500/20"
                  : token === "no"
                  ? "bg-rose-500/15 border-rose-500/40 text-rose-400 shadow-[0_0_24px_rgba(244,63,94,0.08)]"
                  : "bg-transparent border-white/[0.06] text-white/30 hover:text-rose-400/60 hover:border-rose-500/20"
              )}
            >
              {t.toUpperCase()}
            </button>
          ))}
        </div>

        {/* Amount input */}
        <div className={cn(
          "rounded-xl border px-4 py-3.5 transition-all duration-200",
          "bg-white/[0.04] border-white/[0.08] focus-within:border-white/[0.16] focus-within:bg-white/[0.06]"
        )}>
          <p className="text-[10px] uppercase tracking-[0.12em] text-white/30 mb-2">Amount</p>
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
            <span className="text-sm text-white/30 shrink-0">USDC</span>
          </div>
          {quoting && (
            <p className="text-[10px] text-white/30 mt-1.5 animate-pulse">Fetching quote…</p>
          )}
        </div>

        {/* Quote breakdown */}
        {quote && (
          <div className="rounded-xl border border-white/[0.06] bg-white/[0.02] px-4 py-3 space-y-2 animate-in fade-in-0 slide-in-from-top-2 duration-300">
            <QuoteRow label="Estimated out" value={`${formatUsdc(BigInt(quote.amountOut))} ${side === "buy" ? "tokens" : "USDC"}`} highlight />
            <QuoteRow label="Min received" value={`${formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} ${side === "buy" ? "tokens" : "USDC"}`} />
            <QuoteRow label="Price impact" value={`${quote.priceImpactBps / 100}%`} />
            <QuoteRow label="Fee / Slippage" value={`${quote.feeBps / 100}% / ${DEFAULT_SLIPPAGE_BPS / 100}%`} />
            <QuoteRow label="Deadline" value={`${TRADE_DEADLINE_SECS / 60} min`} />
            {simulatedFee && <QuoteRow label="Simulated network fee" value={simulatedFee} />}
          </div>
        )}

        {/* CTA */}
        <button
          onClick={handleTrade}
          disabled={!canTrade}
          className={cn(
            "w-full py-3.5 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)] active:scale-[0.98]",
            canTrade && token === "yes"
              ? "bg-emerald-500/20 border border-emerald-500/35 text-emerald-300 hover:bg-emerald-500/30 shadow-[0_0_30px_rgba(52,211,153,0.07)]"
              : canTrade && token === "no"
              ? "bg-rose-500/20 border border-rose-500/35 text-rose-300 hover:bg-rose-500/30 shadow-[0_0_30px_rgba(244,63,94,0.07)]"
              : "bg-white/[0.04] border border-white/[0.07] text-white/25 cursor-not-allowed"
          )}
        >
          {isPending
            ? "Processing…"
            : quoting
            ? "Quoting…"
            : canTrade
            ? `${side === "buy" ? "Buy" : "Sell"} ${token.toUpperCase()}`
            : "Enter amount"}
        </button>

        <TxStateDisplay state={txState} />
      </div>
    </div>
  );
}

function QuoteRow({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className="flex justify-between text-xs">
      <span className="text-white/35">{label}</span>
      <span className={highlight ? "text-white/80 font-medium" : "text-white/50"}>{value}</span>
    </div>
  );
}

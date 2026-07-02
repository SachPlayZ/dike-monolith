"use client";

import { useState, useTransition, useEffect } from "react";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/contexts/wallet";
import {
  ammQuoteBuyYes,
  ammQuoteBuyNo,
  buildAmmBuyChildYes,
  buildAmmBuyChildNo,
  vaultGetChildAvail,
} from "@/lib/contracts/clients";
import { apiGet } from "@/lib/api/client";
import { normalizeMarketData } from "@/lib/api/normalizers";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState, Outcome, MarketData } from "@/lib/types";

interface ChildTradeFormProps {
  poolId: string;
  currentMarketId: string;
}

const DEFAULT_SLIPPAGE_BPS = 50;
const TRADE_DEADLINE_SECS = 300;

type ChildToken = "yes" | "no";

export function ChildTradeForm({ poolId, currentMarketId }: ChildTradeFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [expanded, setExpanded] = useState(false);
  const [markets, setMarkets] = useState<MarketData[]>([]);
  const [parentMarketId, setParentMarketId] = useState("");
  const [parentOutcome, setParentOutcome] = useState<"Yes" | "No">("Yes");
  const [availableCredit, setAvailableCredit] = useState<string | null>(null);
  const [creditLoading, setCreditLoading] = useState(false);
  const [token, setToken] = useState<ChildToken>("yes");
  const [amountInput, setAmountInput] = useState("");
  const [quote, setQuote] = useState<{ amountOut: string; feeBps: number; priceImpactBps: number } | null>(null);
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  useEffect(() => {
    if (!expanded) return;
    apiGet<{ items: Record<string, unknown>[] }>("/markets")
      .then((res) => {
        const live = res.items
          .map((r) => normalizeMarketData(r))
          .filter((m) => m.status === "Live" && m.marketId !== currentMarketId);
        setMarkets(live);
      })
      .catch(() => {});
  }, [expanded, currentMarketId]);

  useEffect(() => {
    if (!address || !parentMarketId) {
      setAvailableCredit(null);
      return;
    }
    setCreditLoading(true);
    setAvailableCredit(null);
    const timer = setTimeout(async () => {
      try {
        const avail = await vaultGetChildAvail(address, address, parentMarketId, parentOutcome as Outcome);
        setAvailableCredit(avail);
      } catch {
        setAvailableCredit(null);
      } finally {
        setCreditLoading(false);
      }
    }, 400);
    return () => clearTimeout(timer);
  }, [address, parentMarketId, parentOutcome]);

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
        const q =
          token === "yes"
            ? await ammQuoteBuyYes(address, poolId, rawIn)
            : await ammQuoteBuyNo(address, poolId, rawIn);
        setQuote({ amountOut: q.amountOut, feeBps: q.feeBps, priceImpactBps: q.priceImpactBps });
      } catch {
        setQuote(null);
        setSimulatedFee(null);
      } finally {
        setQuoting(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [address, amountInput, token, poolId]);

  async function handleBuy() {
    if (!address || !amountInput || !quote || !parentMarketId) return;
    startTransition(async () => {
      try {
        const rawIn = parseUsdc(amountInput).toString();
        if (availableCredit !== null && BigInt(rawIn) > BigInt(availableCredit)) {
          throw new Error("Requested amount exceeds available parent credit.");
        }
        const minOut = applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS).toString();
        setTxState({ status: "building", hash: null, error: null });
        const xdr =
          token === "yes"
            ? await buildAmmBuyChildYes(address, parentMarketId, parentOutcome as Outcome, poolId, rawIn, minOut, TRADE_DEADLINE_SECS)
            : await buildAmmBuyChildNo(address, parentMarketId, parentOutcome as Outcome, poolId, rawIn, minOut, TRADE_DEADLINE_SECS);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        setQuote(null);
        setAvailableCredit(null);
      } catch (e) {
        const code = parseDikeError(e);
        const message =
          code === "InvalidStatus"
            ? "Parent market is no longer live (resolved, cancelled, or paused) — child credit can't be opened against it."
            : code.includes("exceeds available parent credit")
            ? code
            : code;
        setTxState({ status: "failed", hash: null, error: message });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] px-5 py-4 text-center space-y-3">
        <p className="text-xs text-white/40">Connect wallet to use parent credit</p>
        <button
          onClick={connect}
          className="px-4 py-2 rounded-full bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-300"
        >
          Connect Wallet
        </button>
      </div>
    );
  }

  const exceedsCredit =
    availableCredit !== null &&
    amountInput.length > 0 &&
    (() => {
      try {
        return parseUsdc(amountInput) > BigInt(availableCredit);
      } catch {
        return false;
      }
    })();
  const canBuy = Boolean(quote && amountInput && parentMarketId && !isPending && !quoting && !exceedsCredit);

  return (
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
      {/* Collapsible header */}
      <button
        onClick={() => setExpanded((v) => !v)}
        className="w-full px-5 py-4 flex items-center justify-between border-b border-white/[0.05] hover:bg-white/[0.02] transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">Buy with Parent Credit</p>
          <p className="text-xs text-white/40 mt-0.5">Use parent stake as collateral</p>
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
          {/* Warning */}
          <div className="px-4 py-3 rounded-xl bg-amber-500/[0.07] border border-amber-500/[0.18] text-xs text-amber-300/80">
            Credit encumbers your parent position. Clear child debt before selling parent.
          </div>

          {/* Parent market selector */}
          <div className="space-y-1.5">
            <p className="text-[10px] uppercase tracking-[0.12em] text-white/30">Parent Market</p>
            <div className="rounded-xl bg-white/[0.04] border border-white/[0.08] overflow-hidden">
              <select
                value={parentMarketId}
                onChange={(e) => setParentMarketId(e.target.value)}
                className="w-full bg-transparent px-4 py-3 text-xs text-white/70 outline-none cursor-pointer [&>option]:bg-[#1a1208] [&>option]:text-white"
              >
                <option value="">Select a parent market…</option>
                {markets.map((m) => (
                  <option key={m.marketId} value={m.marketId}>
                    {m.config.question.length > 55
                      ? m.config.question.slice(0, 55) + "…"
                      : m.config.question}
                  </option>
                ))}
              </select>
            </div>
            {markets.length === 0 && expanded && (
              <p className="text-[10px] text-white/25">No other live markets.</p>
            )}
          </div>

          {parentMarketId && (
            <>
              {/* Parent outcome */}
              <div className="space-y-1.5">
                <p className="text-[10px] uppercase tracking-[0.12em] text-white/30">Encumber Outcome</p>
                <div className="grid grid-cols-2 gap-2">
                  {(["Yes", "No"] as const).map((o) => (
                    <button
                      key={o}
                      onClick={() => setParentOutcome(o)}
                      className={cn(
                        "py-2.5 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 border",
                        parentOutcome === o && o === "Yes"
                          ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-400"
                          : parentOutcome === o && o === "No"
                          ? "bg-rose-500/15 border-rose-500/40 text-rose-400"
                          : "bg-transparent border-white/[0.06] text-white/30 hover:text-white/60"
                      )}
                    >
                      {o}
                    </button>
                  ))}
                </div>
              </div>

              {/* Credit display */}
              <div className="px-4 py-3 rounded-xl bg-white/[0.03] border border-white/[0.06]">
                {creditLoading ? (
                  <p className="text-xs text-white/30 animate-pulse">Checking credit…</p>
                ) : availableCredit !== null ? (
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-white/40">Available credit</span>
                    <span className={cn(
                      "text-sm font-semibold font-mono",
                      availableCredit === "0" ? "text-amber-400" : "text-white/80"
                    )}>
                      {formatUsdc(BigInt(availableCredit))} USDC
                    </span>
                  </div>
                ) : (
                  <p className="text-xs text-white/25">Select market to check credit</p>
                )}
              </div>
            </>
          )}

          {/* Buy token */}
          <div className="space-y-1.5">
            <p className="text-[10px] uppercase tracking-[0.12em] text-white/30">Buy Token</p>
            <div className="grid grid-cols-2 gap-2">
              {(["yes", "no"] as ChildToken[]).map((t) => (
                <button
                  key={t}
                  onClick={() => setToken(t)}
                  className={cn(
                    "py-2.5 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 border",
                    t === "yes"
                      ? token === "yes"
                        ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-400"
                        : "bg-transparent border-white/[0.06] text-white/30 hover:text-emerald-400/60"
                      : token === "no"
                      ? "bg-rose-500/15 border-rose-500/40 text-rose-400"
                      : "bg-transparent border-white/[0.06] text-white/30 hover:text-rose-400/60"
                  )}
                >
                  {t.toUpperCase()}
                </button>
              ))}
            </div>
          </div>

          {/* Amount input */}
          <div className={cn(
            "rounded-xl border px-4 py-3.5 transition-all duration-200",
            "bg-white/[0.04] border-white/[0.08] focus-within:border-white/[0.16] focus-within:bg-white/[0.06]"
          )}>
            <p className="text-[10px] uppercase tracking-[0.12em] text-white/30 mb-2">Amount (credit units)</p>
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
            {quoting && <p className="text-[10px] text-white/30 mt-1.5 animate-pulse">Fetching quote…</p>}
            {exceedsCredit && (
              <p className="text-[10px] text-amber-300 mt-1.5">
                Amount exceeds available parent credit.
              </p>
            )}
          </div>

          {/* Quote */}
          {quote && (
            <div className="rounded-xl border border-white/[0.06] bg-white/[0.02] px-4 py-3 space-y-2 animate-in fade-in-0 slide-in-from-top-2 duration-300">
              <div className="flex justify-between text-xs">
                <span className="text-white/35">Estimated out</span>
                <span className="text-white/80 font-medium">{formatUsdc(BigInt(quote.amountOut))} tokens</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-white/35">Min received</span>
                <span className="text-white/50">{formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} tokens</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-white/35">Price impact</span>
                <span className="text-white/50">{quote.priceImpactBps / 100}%</span>
              </div>
              {simulatedFee && (
                <div className="flex justify-between text-xs">
                  <span className="text-white/35">Simulated network fee</span>
                  <span className="text-white/50">{simulatedFee}</span>
                </div>
              )}
            </div>
          )}

          {/* CTA */}
          <button
            onClick={handleBuy}
            disabled={!canBuy}
            className={cn(
              "w-full py-3.5 rounded-xl text-xs font-bold uppercase tracking-widest transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)] active:scale-[0.98]",
              canBuy && token === "yes"
                ? "bg-emerald-500/20 border border-emerald-500/35 text-emerald-300 hover:bg-emerald-500/30"
                : canBuy && token === "no"
                ? "bg-rose-500/20 border border-rose-500/35 text-rose-300 hover:bg-rose-500/30"
                : "bg-white/[0.04] border border-white/[0.07] text-white/25 cursor-not-allowed"
            )}
          >
            {isPending ? "Processing…" : quoting ? "Quoting…" : `Buy ${token.toUpperCase()}`}
          </button>

          <TxStateDisplay state={txState} />
        </div>
      )}
    </div>
  );
}

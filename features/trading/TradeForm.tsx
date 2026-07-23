"use client";

import { useState, useTransition, useEffect } from "react";
import { useRouter } from "next/navigation";
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
import { fetchRawPortfolio } from "@/lib/api/portfolio";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Skeleton } from "@/components/ui/skeleton";
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
  const router = useRouter();
  const { address, isConnected, connect, sign } = useWallet();
  const [side, setSide] = useState<Side>("buy");
  const [token, setToken] = useState<Token>("yes");
  const [amountInput, setAmountInput] = useState("");
  const quoteKey = `${address ?? ""}:${poolId}:${side}:${token}:${amountInput}`;
  const [storedQuote, setQuote] = useState<{
    key: string;
    amountOut: string;
    feeBps: number;
    priceImpactBps: number;
  } | null>(null);
  const quote = storedQuote?.key === quoteKey ? storedQuote : null;
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();
  const [loadedBalances, setBalances] = useState<{
    address: string;
    yes: string;
    no: string;
  } | null>(null);
  const [balancesErrorAddress, setBalancesErrorAddress] = useState<string | null>(null);
  const [balancesRetryNonce, setBalancesRetryNonce] = useState(0);
  const balances = loadedBalances?.address === address ? loadedBalances : null;
  const balancesError = balancesErrorAddress === address;

  useEffect(() => {
    if (!address) return;
    let cancelled = false;
    fetchRawPortfolio(address)
      .then((portfolio) => {
        if (cancelled) return;
        const yesPos = portfolio.positions.find(
          (p) => String(p.market_id) === marketId && p.outcome === "Yes"
        );
        const noPos = portfolio.positions.find(
          (p) => String(p.market_id) === marketId && p.outcome === "No"
        );
        setBalances({
          address,
          yes: String(yesPos?.balance ?? "0"),
          no: String(noPos?.balance ?? "0"),
        });
        setBalancesErrorAddress(null);
      })
      .catch(() => {
        if (cancelled) return;
        setBalancesErrorAddress(address);
      });
    return () => {
      cancelled = true;
    };
  }, [address, marketId, txState.status, balancesRetryNonce]);

  const sellBalance = balances ? BigInt(balances[token]) : null;
  const balancesUnknown = side === "sell" && balancesError && sellBalance === null;
  const noPosition =
    side === "sell" && !balancesUnknown && (sellBalance === null || sellBalance === 0n);
  const exceedsPosition =
    side === "sell" &&
    sellBalance !== null &&
    amountInput.length > 0 &&
    (() => {
      try {
        return parseUsdc(amountInput) > sellBalance;
      } catch {
        return false;
      }
    })();

  useEffect(() => {
    if (!address || !amountInput || parseFloat(amountInput) <= 0) return;
    if (side === "sell" && (noPosition || exceedsPosition)) return;
    const requestKey = quoteKey;
    const timer = setTimeout(async () => {
      setQuoting(true);
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
        setQuote({ key: requestKey, amountOut: q.amountOut, feeBps: q.feeBps, priceImpactBps: q.priceImpactBps });
      } catch (e) {
        setQuote(null);
        setSimulatedFee(null);
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      } finally {
        setQuoting(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [address, amountInput, side, token, poolId, noPosition, exceedsPosition, quoteKey]);

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
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect wallet to trade</p>
          <Button onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
    );
  }

  const isExecuting = isPending || quoting;
  const canTrade = Boolean(
    quote && amountInput && !isExecuting && !noPosition && !exceedsPosition && !balancesUnknown
  );

  return (
    <Card size="sm">
      <CardContent className="space-y-4">
        {/* Header */}
        <div className="pb-4 border-b border-border">
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">Trade</p>
          <p className="text-xs text-foreground/80 line-clamp-2 leading-relaxed font-heading">{marketQuestion}</p>
          <p className="text-[10px] font-mono text-muted-foreground/70 mt-1">#{marketId.slice(0, 8)}…</p>
        </div>

        {/* Buy / Sell tab */}
        <Tabs value={side} onValueChange={(v) => { setSide(v as Side); setQuote(null); }}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="buy">Buy</TabsTrigger>
            <TabsTrigger value="sell">Sell</TabsTrigger>
          </TabsList>
        </Tabs>

        {/* YES / NO selector */}
        <div className="grid grid-cols-2 gap-2">
          {(["yes", "no"] as Token[]).map((t) => (
            <Button
              key={t}
              type="button"
              variant="outline"
              onClick={() => { setToken(t); setQuote(null); }}
              className={cn(
                "h-auto py-3",
                t === "yes"
                  ? token === "yes"
                    ? "border-green-500/40 bg-green-500/10 text-green-700 hover:bg-green-500/15 dark:text-green-400"
                    : "text-muted-foreground hover:text-green-700 hover:border-green-500/30 dark:hover:text-green-400"
                  : token === "no"
                  ? "border-red-500/40 bg-red-500/10 text-red-700 hover:bg-red-500/15 dark:text-red-400"
                  : "text-muted-foreground hover:text-red-700 hover:border-red-500/30 dark:hover:text-red-400"
              )}
            >
              {t.toUpperCase()}
            </Button>
          ))}
        </div>

        {/* Amount input */}
        <div className="rounded-md border border-input px-4 py-3.5 transition-colors focus-within:border-ring">
          <div className="flex items-center justify-between mb-2">
            <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Amount</p>
            {side === "sell" && balances && (
              <button
                type="button"
                onClick={() => setAmountInput(formatUsdc(BigInt(balances[token])))}
                disabled={balances[token] === "0"}
                className="text-[10px] font-bold uppercase tracking-widest text-primary hover:text-primary/80 disabled:text-muted-foreground/40 disabled:cursor-not-allowed transition-colors duration-200"
              >
                Max {formatUsdc(BigInt(balances[token]))}
              </button>
            )}
          </div>
          <div className="flex items-baseline gap-2">
            <input
              aria-label={`${side === "buy" ? "Buy" : "Sell"} ${token.toUpperCase()} amount`}
              inputMode="decimal"
              type="number"
              min="0"
              step="0.01"
              placeholder="0"
              value={amountInput}
              onChange={(e) => setAmountInput(e.target.value)}
              disabled={noPosition}
              className="flex-1 bg-transparent text-2xl font-semibold text-foreground placeholder:text-muted-foreground/40 outline-none disabled:cursor-not-allowed disabled:opacity-40 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none w-full min-w-0"
            />
            <span className="text-sm text-muted-foreground shrink-0">{side === "sell" ? token.toUpperCase() : "USDC"}</span>
          </div>
          {quoting && (
            <p className="text-[10px] text-muted-foreground mt-1.5 animate-pulse">Fetching quote…</p>
          )}
          {balancesUnknown && (
            <p className="text-[10px] text-yellow-700 dark:text-yellow-400 mt-1.5 flex items-center gap-2">
              Couldn&apos;t verify your position — network read failed.
              <button
                type="button"
                onClick={() => setBalancesRetryNonce((n) => n + 1)}
                className="underline hover:no-underline"
              >
                Retry
              </button>
            </p>
          )}
          {!balancesUnknown && noPosition && (
            <p className="text-[10px] text-yellow-700 dark:text-yellow-400 mt-1.5">
              You don&apos;t hold any {token.toUpperCase()} position in this market.
            </p>
          )}
          {!balancesUnknown && !noPosition && exceedsPosition && (
            <p className="text-[10px] text-yellow-700 dark:text-yellow-400 mt-1.5">
              Amount exceeds your {token.toUpperCase()} position ({formatUsdc(sellBalance ?? 0n)}).
            </p>
          )}
        </div>

        {/* Quote breakdown */}
        {quoting && !quote && (
          <div className="rounded-md border border-border px-4 py-3 space-y-2.5">
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-3/4" />
          </div>
        )}
        {quote && (
          <div className="rounded-md border border-border px-4 py-3 space-y-2">
            <QuoteRow label="Estimated out" value={`${formatUsdc(BigInt(quote.amountOut))} ${side === "buy" ? "tokens" : "USDC"}`} highlight />
            <QuoteRow label="Min received" value={`${formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} ${side === "buy" ? "tokens" : "USDC"}`} />
            <QuoteRow label="Price impact" value={`${quote.priceImpactBps / 100}%`} />
            <QuoteRow label="Fee / Slippage" value={`${quote.feeBps / 100}% / ${DEFAULT_SLIPPAGE_BPS / 100}%`} />
            <QuoteRow label="Deadline" value={`${TRADE_DEADLINE_SECS / 60} min`} />
            {simulatedFee && <QuoteRow label="Simulated network fee" value={simulatedFee} />}
          </div>
        )}

        {/* CTA */}
        <Button
          onClick={handleTrade}
          disabled={!canTrade}
          className={cn(
            "w-full h-11",
            canTrade && token === "yes"
              ? "bg-green-600 text-white hover:bg-green-700"
              : canTrade && token === "no"
              ? "bg-red-600 text-white hover:bg-red-700"
              : ""
          )}
          variant={canTrade ? "default" : "outline"}
        >
          {isPending
            ? "Processing…"
            : quoting
            ? "Quoting…"
            : balancesUnknown
            ? "Couldn't verify position"
            : noPosition
            ? "No position to sell"
            : exceedsPosition
            ? "Exceeds your position"
            : canTrade
            ? `${side === "buy" ? "Buy" : "Sell"} ${token.toUpperCase()}`
            : "Enter amount"}
        </Button>

        <TxStateDisplay state={txState} />
      </CardContent>
    </Card>
  );
}

function QuoteRow({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className="flex justify-between text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className={highlight ? "text-foreground font-medium" : "text-muted-foreground"}>{value}</span>
    </div>
  );
}

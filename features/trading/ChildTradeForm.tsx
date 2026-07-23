"use client";

import { useState, useTransition, useEffect } from "react";
import { useRouter } from "next/navigation";
import { ChevronDown } from "lucide-react";
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
import { fetchRawPortfolio } from "@/lib/api/portfolio";
import { normalizeMarketData } from "@/lib/api/normalizers";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TxState, Outcome, MarketData } from "@/lib/types";

interface ChildTradeFormProps {
  poolId: string;
  currentMarketId: string;
}

const DEFAULT_SLIPPAGE_BPS = 50;
const TRADE_DEADLINE_SECS = 300;

type ChildToken = "yes" | "no";

export function ChildTradeForm({ poolId, currentMarketId }: ChildTradeFormProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign } = useWallet();
  const [expanded, setExpanded] = useState(false);
  const [markets, setMarkets] = useState<MarketData[]>([]);
  const marketsKey = `${address ?? ""}:${currentMarketId}`;
  const [loadedMarketsKey, setLoadedMarketsKey] = useState<string | null>(null);
  const marketsLoading = Boolean(expanded && address && loadedMarketsKey !== marketsKey);
  const [positionStakes, setPositionStakes] = useState<Record<string, { yes: string; no: string }>>({});
  const [parentMarketId, setParentMarketId] = useState("");
  const [parentOutcome, setParentOutcome] = useState<"Yes" | "No">("Yes");
  const creditKey = `${address ?? ""}:${parentMarketId}:${parentOutcome}`;
  const [creditResult, setCreditResult] = useState<{ key: string; value: string | null } | null>(null);
  const availableCredit = creditResult?.key === creditKey ? creditResult.value : null;
  const creditLoading = Boolean(address && parentMarketId && creditResult?.key !== creditKey);
  const [token, setToken] = useState<ChildToken>("yes");
  const [amountInput, setAmountInput] = useState("");
  const quoteKey = `${address ?? ""}:${poolId}:${token}:${amountInput}`;
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

  useEffect(() => {
    if (!expanded || !address) return;
    let cancelled = false;
    const requestKey = marketsKey;
    Promise.all([
      apiGet<{ items: Record<string, unknown>[] }>("/markets"),
      fetchRawPortfolio(address),
    ])
      .then(([res, portfolio]) => {
        if (cancelled) return;
        const live = res.items
          .map((r) => normalizeMarketData(r))
          .filter((m) => m.status === "Live" && m.marketId !== currentMarketId);

        const vaultByMarket = new Map(
          portfolio.vaultState.map((v) => [String(v.market_id), v])
        );

        const stakeMap: Record<string, { yes: string; no: string }> = {};
        const held: MarketData[] = [];
        live.forEach((m) => {
          const vault = vaultByMarket.get(m.marketId);
          const yes = String(vault?.root_stake_yes ?? "0");
          const no = String(vault?.root_stake_no ?? "0");
          if (BigInt(yes) > 0n || BigInt(no) > 0n) {
            stakeMap[m.marketId] = { yes, no };
            held.push(m);
          }
        });

        setPositionStakes(stakeMap);
        setMarkets(held);
      })
      .catch(() => {
        if (cancelled) return;
        setPositionStakes({});
        setMarkets([]);
      })
      .finally(() => {
        if (!cancelled) setLoadedMarketsKey(requestKey);
      });
    return () => {
      cancelled = true;
    };
  }, [expanded, address, currentMarketId, marketsKey]);

  useEffect(() => {
    if (!address || !parentMarketId) return;
    const requestKey = creditKey;
    const timer = setTimeout(async () => {
      try {
        const avail = await vaultGetChildAvail(address, address, parentMarketId, parentOutcome as Outcome);
        setCreditResult({ key: requestKey, value: avail });
      } catch {
        setCreditResult({ key: requestKey, value: null });
      }
    }, 400);
    return () => clearTimeout(timer);
  }, [address, parentMarketId, parentOutcome, creditKey]);

  useEffect(() => {
    if (!address || !amountInput || parseFloat(amountInput) <= 0) return;
    const requestKey = quoteKey;
    const timer = setTimeout(async () => {
      setQuoting(true);
      try {
        const rawIn = parseUsdc(amountInput).toString();
        const q =
          token === "yes"
            ? await ammQuoteBuyYes(address, poolId, rawIn)
            : await ammQuoteBuyNo(address, poolId, rawIn);
        setQuote({ key: requestKey, amountOut: q.amountOut, feeBps: q.feeBps, priceImpactBps: q.priceImpactBps });
      } catch {
        setQuote(null);
        setSimulatedFee(null);
      } finally {
        setQuoting(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [address, amountInput, token, poolId, quoteKey]);

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
        setCreditResult(null);
        router.refresh();
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
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-xs text-muted-foreground">Connect wallet to use parent credit</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
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
    <Card size="sm" className="overflow-hidden py-0">
      {/* Collapsible header */}
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
        aria-controls="child-trade-form"
        className="w-full px-5 py-4 flex items-center justify-between border-b border-border hover:bg-muted/50 transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Buy with Parent Credit</p>
          <p className="text-xs text-muted-foreground/80 mt-0.5">Use parent stake as collateral</p>
        </div>
        <ChevronDown className={cn("size-4 text-muted-foreground transition-transform duration-200", expanded && "rotate-180")} />
      </button>

      {expanded && (
        <div id="child-trade-form" className="p-5 space-y-4">
          {/* Warning */}
          <div className="px-4 py-3 rounded-md bg-yellow-500/10 border border-yellow-500/25 text-xs text-yellow-700 dark:text-yellow-400">
            Credit encumbers your parent position. Clear child debt before selling parent.
          </div>

          {/* Parent market selector */}
          <div className="space-y-1.5">
            <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Parent Market</p>
            <Select
              value={parentMarketId}
              onValueChange={(nextId) => {
                setParentMarketId(nextId);
                const stake = positionStakes[nextId];
                if (stake) {
                  setParentOutcome(BigInt(stake.yes) > 0n ? "Yes" : "No");
                }
              }}
            >
              <SelectTrigger className="w-full rounded-md border border-input bg-transparent px-4 py-3 h-auto">
                <SelectValue placeholder="Select a parent market…" />
              </SelectTrigger>
              <SelectContent>
                {markets.map((m) => {
                  const stake = positionStakes[m.marketId];
                  const worth = stake ? BigInt(stake.yes) + BigInt(stake.no) : 0n;
                  const label = m.config.question.length > 40
                    ? m.config.question.slice(0, 40) + "…"
                    : m.config.question;
                  return (
                    <SelectItem key={m.marketId} value={m.marketId}>
                      {label} - {formatUsdc(worth)} USDC position
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
            {marketsLoading && (
              <p className="text-[10px] text-muted-foreground/70">Checking your positions…</p>
            )}
            {!marketsLoading && markets.length === 0 && expanded && (
              <p className="text-[10px] text-muted-foreground/70">
                You don&apos;t hold a position in any other live market.
              </p>
            )}
          </div>

          {parentMarketId && (
            <>
              {/* Parent outcome */}
              <div className="space-y-1.5">
                <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Encumber Outcome</p>
                <div className="grid grid-cols-2 gap-2">
                  {(["Yes", "No"] as const).map((o) => {
                    const stake = positionStakes[parentMarketId];
                    const stakeAmount = stake ? (o === "Yes" ? stake.yes : stake.no) : "0";
                    const hasStake = BigInt(stakeAmount) > 0n;
                    return (
                      <Button
                        key={o}
                        type="button"
                        variant="outline"
                        onClick={() => setParentOutcome(o)}
                        disabled={!hasStake}
                        className={cn(
                          "h-auto py-2.5 flex-col",
                          parentOutcome === o && o === "Yes"
                            ? "border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-400"
                            : parentOutcome === o && o === "No"
                            ? "border-red-500/40 bg-red-500/10 text-red-700 dark:text-red-400"
                            : "text-muted-foreground"
                        )}
                      >
                        {o}
                        {hasStake && (
                          <span className="block text-[9px] font-normal normal-case tracking-normal mt-0.5 opacity-70">
                            {formatUsdc(BigInt(stakeAmount))} USDC
                          </span>
                        )}
                      </Button>
                    );
                  })}
                </div>
              </div>

              {/* Credit display */}
              <div className="px-4 py-3 rounded-md bg-muted/50 border border-border">
                {creditLoading ? (
                  <p className="text-xs text-muted-foreground animate-pulse">Checking credit…</p>
                ) : availableCredit !== null ? (
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Available credit (60% of position)</span>
                    <span className={cn(
                      "text-sm font-semibold font-mono",
                      availableCredit === "0" ? "text-yellow-700 dark:text-yellow-400" : "text-foreground/80"
                    )}>
                      {formatUsdc(BigInt(availableCredit))} USDC
                    </span>
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground/70">Select market to check credit</p>
                )}
              </div>
            </>
          )}

          {/* Buy token */}
          <div className="space-y-1.5">
            <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Buy Token</p>
            <div className="grid grid-cols-2 gap-2">
              {(["yes", "no"] as ChildToken[]).map((t) => (
                <Button
                  key={t}
                  type="button"
                  variant="outline"
                  onClick={() => setToken(t)}
                  className={cn(
                    "h-auto py-2.5",
                    t === "yes"
                      ? token === "yes"
                        ? "border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-400"
                        : "text-muted-foreground"
                      : token === "no"
                      ? "border-red-500/40 bg-red-500/10 text-red-700 dark:text-red-400"
                      : "text-muted-foreground"
                  )}
                >
                  {t.toUpperCase()}
                </Button>
              ))}
            </div>
          </div>

          {/* Amount input */}
          <div className="rounded-md border border-input px-4 py-3.5 transition-colors focus-within:border-ring">
            <div className="flex items-center justify-between mb-2">
              <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground">Amount (credit units)</p>
              {availableCredit !== null && availableCredit !== "0" && (
                <button
                  type="button"
                  onClick={() => setAmountInput(formatUsdc(BigInt(availableCredit)))}
                  className="text-[10px] font-bold uppercase tracking-widest text-primary hover:text-primary/80 transition-colors duration-200"
                >
                  Max {formatUsdc(BigInt(availableCredit))}
                </button>
              )}
            </div>
            <div className="flex items-baseline gap-2">
              <input
                aria-label={`Parent credit amount for ${token.toUpperCase()}`}
                inputMode="decimal"
                type="number"
                min="0"
                step="0.01"
                placeholder="0"
                value={amountInput}
                onChange={(e) => setAmountInput(e.target.value)}
                className="flex-1 bg-transparent text-2xl font-semibold text-foreground placeholder:text-muted-foreground/40 outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none w-full min-w-0"
              />
              <span className="text-sm text-muted-foreground shrink-0">USDC</span>
            </div>
            {quoting && <p className="text-[10px] text-muted-foreground mt-1.5 animate-pulse">Fetching quote…</p>}
            {exceedsCredit && (
              <p className="text-[10px] text-yellow-700 dark:text-yellow-400 mt-1.5">
                Amount exceeds available parent credit.
              </p>
            )}
          </div>

          {/* Quote */}
          {quoting && !quote && (
            <div className="rounded-md border border-border px-4 py-3 space-y-2.5">
              <Skeleton className="h-3 w-full" />
              <Skeleton className="h-3 w-full" />
              <Skeleton className="h-3 w-2/3" />
            </div>
          )}
          {quote && (
            <div className="rounded-md border border-border px-4 py-3 space-y-2">
              <div className="flex justify-between text-xs">
                <span className="text-muted-foreground">Estimated out</span>
                <span className="text-foreground font-medium">{formatUsdc(BigInt(quote.amountOut))} tokens</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-muted-foreground">Min received</span>
                <span className="text-muted-foreground">{formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} tokens</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-muted-foreground">Price impact</span>
                <span className="text-muted-foreground">{quote.priceImpactBps / 100}%</span>
              </div>
              {simulatedFee && (
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Simulated network fee</span>
                  <span className="text-muted-foreground">{simulatedFee}</span>
                </div>
              )}
            </div>
          )}

          {/* CTA */}
          <Button
            className={cn(
              "w-full h-11",
              canBuy && token === "yes"
                ? "bg-green-600 text-white hover:bg-green-700"
                : canBuy && token === "no"
                ? "bg-red-600 text-white hover:bg-red-700"
                : ""
            )}
            variant={canBuy ? "default" : "outline"}
            onClick={handleBuy}
            disabled={!canBuy}
          >
            {isPending ? "Processing…" : quoting ? "Quoting…" : `Buy ${token.toUpperCase()}`}
          </Button>

          <TxStateDisplay state={txState} />
        </div>
      )}
    </Card>
  );
}

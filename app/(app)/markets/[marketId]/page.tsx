import { Suspense } from "react";
import { notFound } from "next/navigation";
import { fetchMarket } from "@/lib/api/markets";
import { fetchMarketResolution } from "@/lib/api/resolution";
import { MarketStatusBadge } from "@/features/market/MarketStatusBadge";
import { TradeForm } from "@/features/trading/TradeForm";
import { LiquidityForm } from "@/features/trading/LiquidityForm";
import { ChildTradeForm } from "@/features/trading/ChildTradeForm";
import { UserPositionPanel } from "@/features/trading/UserPositionPanel";
import { CloseTradingButton } from "@/features/market/CloseTradingButton";
import { ResolutionPanel } from "@/features/resolution/ResolutionPanel";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { LiveRefresh } from "@/components/data-state/LiveRefresh";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ServiceUnavailableError } from "@/lib/api/client";
import { formatUsdc, impliedYesBps } from "@/lib/stellar/scval";
import { cn } from "@/lib/utils";
import { NetworkMismatchError } from "@/lib/stellar/config";
import { safeReferenceUrl } from "@/lib/validation/reference-url";

interface PageProps {
  params: Promise<{ marketId: string }>;
}

export async function generateMetadata({ params }: PageProps) {
  const { marketId } = await params;
  return { title: `Market ${marketId.slice(0, 8)}… — DIKE` };
}

async function MarketDetailContent({ marketId }: { marketId: string }) {
  let market, resolution;
  try {
    [market, resolution] = await Promise.all([
      fetchMarket(marketId),
      fetchMarketResolution(marketId).catch(() => ({ marketId, request: null })),
    ]);
  } catch (e) {
    if (e instanceof ServiceUnavailableError) {
      return (
        <Alert>
          <AlertDescription>
            Market data is temporarily unavailable. Please try again.
          </AlertDescription>
        </Alert>
      );
    }
    if (e instanceof NetworkMismatchError) {
      return (
        <Alert variant="destructive">
          <AlertDescription>{e.message}</AlertDescription>
        </Alert>
      );
    }
    notFound();
  }

  const isExpired = market.status === "Live" && !market.tradeable;
  const isTradeable = market.tradeable;
  const yesBps = impliedYesBps(market.yesReserve, market.noReserve);
  const yesPercent = yesBps / 100;
  const noPercent = 100 - yesPercent;
  const expiry = new Date(market.config.expiry * 1000);
  const hasReserves = market.yesReserve !== "0" || market.noReserve !== "0";
  const rulesUrl = safeReferenceUrl(market.config.rulesUri);

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      <LiveRefresh marketId={Number(market.marketId)} />
      {/* ── Left: Market Info ── */}
      <div className="lg:col-span-2 space-y-4">

        {/* Market question card */}
        <Card size="sm">
          <CardContent>
          <div className="flex items-start gap-3 justify-between mb-4">
            <div className="flex items-center gap-2 flex-wrap">
              <Badge variant="secondary" className="px-2.5 py-1 bg-muted rounded-full normal-case tracking-[0.1em]">
                {market.config.category}
              </Badge>
              <MarketStatusBadge status={market.status} />
            </div>
          </div>

          <h1 className="font-heading text-xl md:text-2xl font-normal leading-snug text-foreground mb-5">
            {market.config.question}
          </h1>

          {market.finalOutcome && (
            <div className={cn(
              "mb-5 px-4 py-3 rounded-md border text-sm font-semibold",
              market.finalOutcome === "Yes"
                ? "bg-green-500/10 border-green-500/30 text-green-700 dark:text-green-400"
                : market.finalOutcome === "No"
                ? "bg-red-500/10 border-red-500/30 text-red-700 dark:text-red-400"
                : "bg-muted/50 border-border text-muted-foreground"
            )}>
              Final Outcome: {market.finalOutcome}
            </div>
          )}

          {/* Meta grid */}
          <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
            {[
              { label: "Expires", value: expiry.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" }) },
              { label: "Bond", value: formatUsdc(BigInt(market.config.bondAmount)) + " USDC" },
              { label: "Dispute window", value: `${market.config.disputeWindow / 3600}h` },
              { label: "Collateral", value: market.config.collateral.slice(0, 8) + "…" },
              { label: "Creator", value: market.config.creator.slice(0, 8) + "…" },
            ].map(({ label, value }) => (
              <div key={label} className="px-3 py-2.5 rounded-md bg-muted/50 border border-border">
                <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground mb-1">{label}</p>
                <p className="text-xs font-medium text-foreground/80 font-mono">{value}</p>
              </div>
            ))}
          </div>

          {rulesUrl && (
            <a
              href={rulesUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 mt-4 text-xs text-primary hover:text-primary/80 transition-colors duration-200"
            >
              View Rules ↗
            </a>
          )}
          {market.config.rulesUri && !rulesUrl && (
            <Alert variant="warning" className="mt-4">
              <AlertDescription>
                This market&apos;s rules URL is not a safe, public HTTPS link. Verify the on-chain rules hash before relying on it.
              </AlertDescription>
            </Alert>
          )}
          </CardContent>
        </Card>

        {/* Pool card */}
        {market.poolId && (
          <Card size="sm" className="overflow-hidden py-0">
            <div className="px-6 pt-5 pb-4 border-b border-border">
              <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">AMM Pool</h2>
            </div>
            <div className="px-6 py-5 space-y-5">
              {/* Probability bar */}
              <div className="space-y-2">
                <div className="h-2 rounded-full overflow-hidden flex">
                  <div
                    className="bg-green-500 transition-[width] duration-1000 ease-[cubic-bezier(0.32,0.72,0,1)]"
                    style={{ width: `${yesPercent}%` }}
                  />
                  <div className="flex-1 bg-red-500" />
                </div>
                <div className="flex justify-between text-xs">
                  <div className="flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                    <span className="text-green-700 dark:text-green-400 font-semibold">YES</span>
                    <span className="text-muted-foreground font-mono">{yesPercent.toFixed(1)}¢</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-muted-foreground font-mono">{noPercent.toFixed(1)}¢</span>
                    <span className="text-red-700 dark:text-red-400 font-semibold">NO</span>
                    <span className="w-1.5 h-1.5 rounded-full bg-red-500" />
                  </div>
                </div>
              </div>

              {/* Reserve amounts */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="px-4 py-3 rounded-md bg-green-500/10 border border-green-500/20">
                  <p className="text-[10px] uppercase tracking-[0.12em] text-green-700/70 dark:text-green-400/60 mb-1">YES Reserve</p>
                  <p className="text-sm font-semibold font-mono text-green-700 dark:text-green-400">
                    {hasReserves ? formatUsdc(BigInt(market.yesReserve)) : "—"} <span className="text-xs font-normal opacity-70">USDC</span>
                  </p>
                </div>
                <div className="px-4 py-3 rounded-md bg-red-500/10 border border-red-500/20">
                  <p className="text-[10px] uppercase tracking-[0.12em] text-red-700/70 dark:text-red-400/60 mb-1">NO Reserve</p>
                  <p className="text-sm font-semibold font-mono text-red-700 dark:text-red-400">
                    {hasReserves ? formatUsdc(BigInt(market.noReserve)) : "—"} <span className="text-xs font-normal opacity-70">USDC</span>
                  </p>
                </div>
              </div>
            </div>
          </Card>
        )}

        {/* Resolution */}
        {(market.status === "TradingClosed" ||
          market.status === "ResolutionRequested" ||
          market.status === "Proposed" ||
          market.status === "Disputed") && (
          <Card size="sm" className="overflow-hidden py-0">
            <div className="px-6 pt-5 pb-4 border-b border-border">
              <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Resolution</h2>
            </div>
            <div className="px-6 py-5">
              <ResolutionPanel market={market} request={resolution?.request ?? null} />
            </div>
          </Card>
        )}
      </div>

      {/* ── Right: Trading ── */}
      <div className="space-y-4">
        <UserPositionPanel marketId={market.marketId} poolId={market.poolId} />
        {isTradeable && market.poolId ? (
          <>
            <TradeForm
              marketId={market.marketId}
              poolId={market.poolId}
              marketQuestion={market.config.question}
            />
            <ChildTradeForm poolId={market.poolId} currentMarketId={market.marketId} />
          </>
        ) : market.status === "Live" && isExpired ? (
          <>
            <Alert variant="warning">
              <AlertDescription>
                This market has expired. Trading is disabled while its on-chain status awaits closure.
              </AlertDescription>
            </Alert>
            <CloseTradingButton marketId={market.marketId} isExpired />
          </>
        ) : !market.poolId ? (
          <Card size="sm">
            <CardContent className="text-center">
              <p className="text-sm text-muted-foreground">
                Trading not available - market is <span className="text-foreground/80 font-medium">{market.status}</span>.
              </p>
            </CardContent>
          </Card>
        ) : null}
        {/* Claiming accrued LP fees isn't gated by trading status on-chain
            (claim_lp_fees has no market-status check), so this stays mounted
            across every status, not just Live — add/remove liquidity inside
            it still enforce their own on-chain status gates. */}
        {market.poolId && (
          <LiquidityForm
            poolId={market.poolId}
            yesReserve={market.yesReserve}
            noReserve={market.noReserve}
            totalLpShares={market.totalLpShares}
          />
        )}
      </div>
    </div>
  );
}

export default async function MarketDetailPage({ params }: PageProps) {
  const { marketId } = await params;
  return (
    <Suspense fallback={<PageLoader />}>
      <MarketDetailContent marketId={marketId} />
    </Suspense>
  );
}

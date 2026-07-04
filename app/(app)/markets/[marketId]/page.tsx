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
import { Alert, AlertDescription } from "@/components/ui/alert";
import { ServiceUnavailableError } from "@/lib/api/client";
import { formatUsdc, impliedYesBps } from "@/lib/stellar/scval";
import { cn } from "@/lib/utils";

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
            dike-services is not running. Start it to view market details.
          </AlertDescription>
        </Alert>
      );
    }
    notFound();
  }

  const isExpired = Date.now() / 1000 >= market.config.expiry;
  const isTradeable = market.status === "Live" && !isExpired;
  const yesBps = impliedYesBps(market.yesReserve, market.noReserve);
  const yesPercent = yesBps / 100;
  const noPercent = 100 - yesPercent;
  const expiry = new Date(market.config.expiry * 1000);
  const hasReserves = market.yesReserve !== "0" || market.noReserve !== "0";

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      {/* ── Left: Market Info ── */}
      <div className="lg:col-span-2 space-y-4">

        {/* Market question card */}
        <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
          <div className="px-6 pt-6 pb-5">
            <div className="flex items-start gap-3 justify-between mb-4">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="px-2.5 py-1 rounded-full bg-white/[0.06] border border-white/[0.08] text-[10px] font-semibold uppercase tracking-[0.15em] text-white/50">
                  {market.config.category}
                </span>
                <MarketStatusBadge status={market.status} />
              </div>
            </div>

            <h1 className="font-heading text-xl md:text-2xl font-normal leading-snug text-white/90 mb-5">
              {market.config.question}
            </h1>

            {market.finalOutcome && (
              <div className={cn(
                "mb-5 px-4 py-3 rounded-xl border text-sm font-semibold",
                market.finalOutcome === "Yes"
                  ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-400"
                  : market.finalOutcome === "No"
                  ? "bg-rose-500/10 border-rose-500/30 text-rose-400"
                  : "bg-white/[0.05] border-white/[0.10] text-white/60"
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
                <div key={label} className="px-3 py-2.5 rounded-xl bg-white/[0.03] border border-white/[0.05]">
                  <p className="text-[10px] uppercase tracking-[0.12em] text-white/30 mb-1">{label}</p>
                  <p className="text-xs font-medium text-white/70 font-mono">{value}</p>
                </div>
              ))}
            </div>

            {market.config.rulesUri && (
              <a
                href={market.config.rulesUri}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5 mt-4 text-xs text-orange-400/80 hover:text-orange-300 transition-colors duration-200"
              >
                View Rules ↗
              </a>
            )}
          </div>
        </div>

        {/* Pool card */}
        {market.poolId && (
          <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
            <div className="px-6 pt-5 pb-4 border-b border-white/[0.05]">
              <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">AMM Pool</p>
            </div>
            <div className="px-6 py-5 space-y-5">
              {/* Probability bar */}
              <div className="space-y-2">
                <div className="h-2 rounded-full overflow-hidden flex">
                  <div
                    className="bg-gradient-to-r from-emerald-600 to-emerald-400 transition-all duration-1000 ease-[cubic-bezier(0.32,0.72,0,1)]"
                    style={{ width: `${yesPercent}%` }}
                  />
                  <div className="flex-1 bg-gradient-to-r from-rose-400 to-rose-600" />
                </div>
                <div className="flex justify-between text-xs">
                  <div className="flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    <span className="text-emerald-400 font-semibold">YES</span>
                    <span className="text-white/40 font-mono">{yesPercent.toFixed(1)}¢</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-white/40 font-mono">{noPercent.toFixed(1)}¢</span>
                    <span className="text-rose-400 font-semibold">NO</span>
                    <span className="w-1.5 h-1.5 rounded-full bg-rose-400" />
                  </div>
                </div>
              </div>

              {/* Reserve amounts */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="px-4 py-3 rounded-xl bg-emerald-500/[0.06] border border-emerald-500/[0.12]">
                  <p className="text-[10px] uppercase tracking-[0.12em] text-emerald-600/60 dark:text-emerald-400/50 mb-1">YES Reserve</p>
                  <p className="text-sm font-semibold font-mono text-emerald-400">
                    {hasReserves ? formatUsdc(BigInt(market.yesReserve)) : "—"} <span className="text-xs font-normal text-emerald-400/50">USDC</span>
                  </p>
                </div>
                <div className="px-4 py-3 rounded-xl bg-rose-500/[0.06] border border-rose-500/[0.12]">
                  <p className="text-[10px] uppercase tracking-[0.12em] text-rose-400/50 mb-1">NO Reserve</p>
                  <p className="text-sm font-semibold font-mono text-rose-400">
                    {hasReserves ? formatUsdc(BigInt(market.noReserve)) : "—"} <span className="text-xs font-normal text-rose-400/50">USDC</span>
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Resolution */}
        {(market.status === "TradingClosed" ||
          market.status === "ResolutionRequested" ||
          market.status === "Proposed" ||
          market.status === "Disputed") && (
          <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
            <div className="px-6 pt-5 pb-4 border-b border-white/[0.05]">
              <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">Resolution</p>
            </div>
            <div className="px-6 py-5">
              <ResolutionPanel market={market} request={resolution?.request ?? null} />
            </div>
          </div>
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
          <CloseTradingButton marketId={market.marketId} expiry={market.config.expiry} />
        ) : !market.poolId ? (
          <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] px-5 py-6 text-center">
            <p className="text-sm text-white/40">
              Trading not available — market is <span className="text-white/60 font-medium">{market.status}</span>.
            </p>
          </div>
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

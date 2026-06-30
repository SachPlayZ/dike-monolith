import { Suspense } from "react";
import { notFound } from "next/navigation";
import { fetchMarket } from "@/lib/api/markets";
import { fetchMarketResolution } from "@/lib/api/resolution";
import { MarketStatusBadge } from "@/features/market/MarketStatusBadge";
import { TradeForm } from "@/features/trading/TradeForm";
import { LiquidityForm } from "@/features/trading/LiquidityForm";
import { ChildTradeForm } from "@/features/trading/ChildTradeForm";
import { ResolutionPanel } from "@/features/resolution/ResolutionPanel";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { ServiceUnavailableError } from "@/lib/api/client";
import { formatUsdc, impliedYesBps } from "@/lib/stellar/scval";

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

  const isTradeable = market.status === "Live";
  const yesBps = impliedYesBps(market.yesReserve, market.noReserve);
  const expiry = new Date(market.config.expiry * 1000);

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      {/* Left: Market Info */}
      <div className="lg:col-span-2 space-y-6">
        <Card size="sm">
          <CardContent className="space-y-4">
            <div className="flex items-start gap-3 justify-between">
              <h1 className="text-lg font-semibold leading-snug">
                {market.config.question}
              </h1>
              <MarketStatusBadge status={market.status} />
            </div>

            <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-xs">
              <Info label="Category" value={market.config.category} />
              <Info
                label="Expiry"
                value={expiry.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}
              />
              <Info label="Collateral" value={market.config.collateral.slice(0, 10) + "…"} />
              <Info label="Bond amount" value={formatUsdc(BigInt(market.config.bondAmount)) + " USDC"} />
              <Info label="Dispute window" value={`${market.config.disputeWindow / 3600}h`} />
              <Info label="Creator" value={market.config.creator.slice(0, 8) + "…"} />
            </div>

            {market.config.rulesUri && (
              <a
                href={market.config.rulesUri}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary underline"
              >
                View Rules
              </a>
            )}

            {market.finalOutcome && (
              <Alert className={
                market.finalOutcome === "Yes"
                  ? "border-green-500/30 text-green-700 dark:text-green-400 after:bg-green-500"
                  : market.finalOutcome === "No"
                  ? "border-red-500/30 text-red-700 dark:text-red-400 after:bg-red-500"
                  : ""
              }>
                <AlertDescription className="font-semibold">
                  Final Outcome: {market.finalOutcome}
                </AlertDescription>
              </Alert>
            )}
          </CardContent>
        </Card>

        {/* AMM Pool */}
        {market.poolId && (
          <Card size="sm">
            <CardHeader className="pb-0">
              <CardTitle className="text-sm font-semibold normal-case tracking-normal">Pool</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex gap-6 text-xs">
                <div>
                  <p className="text-muted-foreground">YES reserve</p>
                  <p className="font-medium text-green-600 dark:text-green-400">
                    {formatUsdc(BigInt(market.yesReserve))} USDC
                  </p>
                </div>
                <div>
                  <p className="text-muted-foreground">NO reserve</p>
                  <p className="font-medium text-red-600 dark:text-red-400">
                    {formatUsdc(BigInt(market.noReserve))} USDC
                  </p>
                </div>
              </div>
              {isTradeable && (
                <div className="flex gap-4 text-xs">
                  <span className="text-green-600 dark:text-green-400">
                    YES {(yesBps / 100).toFixed(1)}¢
                  </span>
                  <span className="text-red-600 dark:text-red-400">
                    NO {((10000 - yesBps) / 100).toFixed(1)}¢
                  </span>
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* Resolution */}
        {(market.status === "TradingClosed" ||
          market.status === "ResolutionRequested" ||
          market.status === "Proposed" ||
          market.status === "Disputed") && (
          <Card size="sm">
            <CardHeader className="pb-0">
              <CardTitle className="text-sm font-semibold normal-case tracking-normal">Resolution</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <ResolutionPanel market={market} request={resolution?.request ?? null} />
            </CardContent>
          </Card>
        )}
      </div>

      {/* Right: Trade + Liquidity */}
      <div className="space-y-4">
        {isTradeable && market.poolId ? (
          <>
            <TradeForm
              marketId={market.marketId}
              poolId={market.poolId}
              marketQuestion={market.config.question}
            />
            <ChildTradeForm poolId={market.poolId} />
            <LiquidityForm poolId={market.poolId} />
          </>
        ) : (
          <Card size="sm">
            <CardContent className="text-sm text-muted-foreground text-center">
              Trading not available — market is {market.status}.
            </CardContent>
          </Card>
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

function Info({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-muted-foreground">{label}</p>
      <p className="font-medium">{value}</p>
    </div>
  );
}

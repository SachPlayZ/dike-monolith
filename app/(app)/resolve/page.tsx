import { Suspense } from "react";
import { fetchMarkets } from "@/lib/api/markets";
import { fetchMarketResolution } from "@/lib/api/resolution";
import { MarketStatusBadge } from "@/features/market/MarketStatusBadge";
import { ResolutionPanel } from "@/features/resolution/ResolutionPanel";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { EmptyState } from "@/components/data-state/EmptyState";
import { ServiceUnavailableError } from "@/lib/api/client";
import type { MarketData, MarketStatus } from "@/lib/types";

export const metadata = {
  title: "Resolve — DIKE",
};

const RESOLUTION_STATUSES: MarketStatus[] = [
  "TradingClosed",
  "ResolutionRequested",
  "Proposed",
  "Disputed",
];

async function ResolutionWorkbench() {
  let markets: MarketData[] = [];
  let error: string | null = null;

  try {
    const all = await fetchMarkets();
    markets = all.filter((m) => RESOLUTION_STATUSES.includes(m.status));
  } catch (e) {
    error =
      e instanceof ServiceUnavailableError
        ? "dike-services is not running."
        : e instanceof Error
        ? e.message
        : "Failed to load markets";
  }

  if (error) {
    return <EmptyState title="Unavailable" description={error} />;
  }

  if (markets.length === 0) {
    return (
      <EmptyState
        title="No markets pending resolution"
        description="Markets awaiting resolution will appear here."
      />
    );
  }

  return (
    <div className="space-y-6">
      {markets.map(async (market) => {
        const resolution = await fetchMarketResolution(market.marketId).catch(() => ({
          marketId: market.marketId,
          request: null,
        }));

        return (
          <div key={market.marketId} className="rounded-lg border border-border p-5 space-y-4">
            <div className="flex items-start justify-between gap-3">
              <p className="text-sm font-medium">{market.config.question}</p>
              <MarketStatusBadge status={market.status} />
            </div>
            <ResolutionPanel market={market} request={resolution.request} />
          </div>
        );
      })}
    </div>
  );
}

export default function ResolvePage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Resolution Workbench</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Request, propose, dispute, and finalize market outcomes.
        </p>
      </div>

      <Suspense fallback={<PageLoader />}>
          <ResolutionWorkbench />
      </Suspense>
    </div>
  );
}

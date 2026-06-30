import { Suspense } from "react";
import { fetchMarkets } from "@/lib/api/markets";
import { MarketCard } from "@/features/market/MarketCard";
import { MarketFilters } from "@/features/market/MarketFilters";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { EmptyState } from "@/components/data-state/EmptyState";
import { ServiceUnavailableError } from "@/lib/api/client";
import type { MarketData } from "@/lib/types";

export const metadata = {
  title: "Markets — DIKE",
};

interface PageProps {
  searchParams: Promise<{ status?: string; category?: string }>;
}

async function MarketList({ statusFilter }: { statusFilter: string }) {
  let markets: MarketData[] = [];
  let error: string | null = null;

  try {
    markets = await fetchMarkets();
  } catch (e) {
    if (e instanceof ServiceUnavailableError) {
      error = "The markets service is currently unavailable. Please try again later.";
    } else {
      error = e instanceof Error ? e.message : "Failed to load markets";
    }
  }

  if (error) {
    return (
      <EmptyState
        title="Markets unavailable"
        description={error}
      />
    );
  }

  const filtered = statusFilter
    ? markets.filter((m) => m.status === statusFilter)
    : markets;

  if (filtered.length === 0) {
    return (
      <EmptyState
        title="No markets found"
        description={statusFilter ? `No markets with status "${statusFilter}".` : "No prediction markets yet."}
      />
    );
  }

  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {filtered.map((m) => (
        <MarketCard key={m.marketId} market={m} />
      ))}
    </div>
  );
}

export default async function PredictionsPage({ searchParams }: PageProps) {
  const { status: statusFilter = "" } = await searchParams;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">Prediction Markets</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Browse and trade on live prediction markets.
        </p>
      </div>

      <Suspense>
        <MarketFilters />
      </Suspense>

      <Suspense fallback={<PageLoader />}>
        <MarketList statusFilter={statusFilter} />
      </Suspense>
    </div>
  );
}

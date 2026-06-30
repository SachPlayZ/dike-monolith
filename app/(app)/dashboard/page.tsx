"use client";

import { useState, useEffect } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchPortfolio } from "@/lib/api/portfolio";
import { PositionCard } from "@/features/portfolio/PositionCard";
import { RedeemForm } from "@/features/portfolio/RedeemForm";
import { EmptyState } from "@/components/data-state/EmptyState";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { ServiceUnavailableError } from "@/lib/api/client";
import type { UserPosition } from "@/lib/types";

export default function DashboardPage() {
  const { address, isConnected, connect } = useWallet();
  const [positions, setPositions] = useState<UserPosition[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedPosition, setSelectedPosition] = useState<UserPosition | null>(null);

  useEffect(() => {
    if (!address) return;
    setLoading(true);
    setError(null);
    fetchPortfolio(address)
      .then(setPositions)
      .catch((e) => {
        setError(
          e instanceof ServiceUnavailableError
            ? "dike-services is not running. Start it to view your portfolio."
            : e instanceof Error
            ? e.message
            : "Failed to load portfolio"
        );
      })
      .finally(() => setLoading(false));
  }, [address]);

  if (!isConnected) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold tracking-tight">Portfolio</h1>
        <EmptyState
          title="Connect your wallet"
          description="Connect to view your positions, balances, and redeemable outcomes."
          action={
            <button
              onClick={connect}
              className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
            >
              Connect Wallet
            </button>
          }
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Portfolio</h1>
        <p className="text-sm text-muted-foreground mt-1 font-mono">
          {address?.slice(0, 8)}…{address?.slice(-8)}
        </p>
      </div>

      {loading && <PageLoader />}

      {error && (
        <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
          {error}
        </div>
      )}

      {!loading && !error && positions.length === 0 && (
        <EmptyState
          title="No positions"
          description="You have no open positions. Trade on a market to get started."
        />
      )}

      {!loading && positions.length > 0 && (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {positions.map((pos) => (
            <PositionCard
              key={pos.marketId}
              position={pos}
              onRedeem={setSelectedPosition}
            />
          ))}
        </div>
      )}

      {selectedPosition && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 px-4">
          <div className="w-full max-w-md rounded-lg border border-border bg-background p-6 space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold">Redeem Position</h2>
              <button
                onClick={() => setSelectedPosition(null)}
                className="text-muted-foreground hover:text-foreground text-xs"
              >
                ✕ Close
              </button>
            </div>
            <p className="text-xs text-muted-foreground line-clamp-2">
              {selectedPosition.question}
            </p>
            <RedeemForm
              position={selectedPosition}
              onSuccess={() => {
                setSelectedPosition(null);
                if (address) fetchPortfolio(address).then(setPositions).catch(() => {});
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}

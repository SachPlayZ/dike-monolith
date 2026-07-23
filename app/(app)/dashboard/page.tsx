"use client";

import { useState, useEffect, useRef } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchPortfolio } from "@/lib/api/portfolio";
import { hydratePortfolioPositions } from "@/lib/portfolio/live";
import { useLiveUpdates } from "@/lib/hooks/useLiveUpdates";
import { PositionCard } from "@/features/portfolio/PositionCard";
import { RedeemForm } from "@/features/portfolio/RedeemForm";
import { EmptyState } from "@/components/data-state/EmptyState";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ServiceUnavailableError } from "@/lib/api/client";
import type { UserPosition } from "@/lib/types";

export default function DashboardPage() {
  const { address, isConnected, connect } = useWallet();
  const [portfolioAddress, setPortfolioAddress] = useState<string | null>(null);
  const [positions, setPositions] = useState<UserPosition[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedPosition, setSelectedPosition] = useState<UserPosition | null>(null);

  async function refreshPortfolio(nextAddress: string) {
    try {
      const nextPositions = await fetchPortfolio(nextAddress);
      const livePositions = await hydratePortfolioPositions(nextAddress, nextPositions);
      setPositions(livePositions);
      setError(null);
    } catch (e) {
      setPositions([]);
      setError(
        e instanceof ServiceUnavailableError
          ? "Portfolio data is temporarily unavailable. Please try again."
          : e instanceof Error
          ? e.message
          : "Failed to load portfolio"
      );
    } finally {
      setPortfolioAddress(nextAddress);
    }
  }

  useEffect(() => {
    if (!address) return;
    let cancelled = false;
    void fetchPortfolio(address)
      .then((nextPositions) => hydratePortfolioPositions(address, nextPositions))
      .then((nextPositions) => {
        if (cancelled) return;
        setPositions(nextPositions);
        setError(null);
        setPortfolioAddress(address);
      })
      .catch((e) => {
        if (cancelled) return;
        setPositions([]);
        setError(
          e instanceof ServiceUnavailableError
            ? "Portfolio data is temporarily unavailable. Please try again."
            : e instanceof Error
            ? e.message
            : "Failed to load portfolio"
        );
        setPortfolioAddress(address);
      });

    return () => {
      cancelled = true;
    };
  }, [address]);

  const pendingLiveRefreshRef = useRef(false);
  useLiveUpdates((update) => {
    if (
      !address ||
      update.type !== "portfolio" ||
      update.address.toUpperCase() !== address.toUpperCase() ||
      pendingLiveRefreshRef.current
    ) {
      return;
    }
    pendingLiveRefreshRef.current = true;
    setTimeout(() => {
      pendingLiveRefreshRef.current = false;
      void refreshPortfolio(address);
    }, 300);
  });

  const loading = Boolean(address) && portfolioAddress !== address;

  if (!isConnected) {
    return (
      <div className="space-y-6">
        <h1 className="font-heading text-3xl font-normal tracking-tight">Portfolio</h1>
        <EmptyState
          title="Connect your wallet"
          description="Connect to view your positions, balances, and redeemable outcomes."
          action={<Button size="sm" onClick={connect}>Connect Wallet</Button>}
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">Portfolio</h1>
        <p className="text-sm text-muted-foreground mt-1 font-mono">
          {address?.slice(0, 8)}…{address?.slice(-8)}
        </p>
      </div>

      {loading && <PageLoader />}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
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

      <Dialog
        open={!!selectedPosition}
        onOpenChange={(open) => !open && setSelectedPosition(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Redeem Position</DialogTitle>
          </DialogHeader>
          {selectedPosition && (
            <>
              <p className="text-xs text-muted-foreground line-clamp-2">
                {selectedPosition.question}
              </p>
              <RedeemForm
                position={selectedPosition}
                onSuccess={() => {
                  setSelectedPosition(null);
                  if (address) {
                    void refreshPortfolio(address);
                  }
                }}
              />
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

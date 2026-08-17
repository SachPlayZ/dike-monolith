"use client";

import { useState, useEffect } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchRawPortfolio } from "@/lib/api/portfolio";
import { formatUsdc } from "@/lib/stellar/scval";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface PositionData {
  yesBalance: string;
  noBalance: string;
  lpShares: string;
  rootStakeYes: string;
  rootStakeNo: string;
  deposit: string;
}

interface UserPositionPanelProps {
  marketId: string;
  poolId: string | null;
}

export function UserPositionPanel({ marketId, poolId }: UserPositionPanelProps) {
  const { address, isConnected } = useWallet();
  const [position, setPosition] = useState<PositionData | null>(null);
  const [hasFetchError, setHasFetchError] = useState(false);
  const [retryNonce, setRetryNonce] = useState(0);

  useEffect(() => {
    if (!address) return;
    let cancelled = false;
    fetchRawPortfolio(address)
      .then((portfolio) => {
        if (cancelled) return;
        setHasFetchError(false);
        const vault = portfolio.vaultState.find((v) => String(v.market_id) === marketId);
        const yesPos = portfolio.positions.find(
          (p) => String(p.market_id) === marketId && p.outcome === "Yes"
        );
        const noPos = portfolio.positions.find(
          (p) => String(p.market_id) === marketId && p.outcome === "No"
        );
        const lp = poolId
          ? portfolio.lpPositions.find((p) => String(p.pool_id) === poolId)
          : undefined;
        setPosition({
          yesBalance: String(yesPos?.balance ?? "0"),
          noBalance: String(noPos?.balance ?? "0"),
          lpShares: String(lp?.shares ?? "0"),
          rootStakeYes: String(vault?.root_stake_yes ?? "0"),
          rootStakeNo: String(vault?.root_stake_no ?? "0"),
          deposit: String(vault?.user_deposit ?? "0"),
        });
      })
      .catch(() => {
        if (cancelled) return;
        setHasFetchError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [address, marketId, poolId, retryNonce]);

  if (!isConnected) return null;
  if (!position && !hasFetchError) return null;

  const hasPosition = Boolean(
    position &&
      (position.yesBalance !== "0" ||
        position.noBalance !== "0" ||
        position.lpShares !== "0" ||
        position.rootStakeYes !== "0" ||
        position.rootStakeNo !== "0" ||
        position.deposit !== "0")
  );
  if (!hasPosition && !hasFetchError) return null;

  return (
    <Card size="sm" className="overflow-hidden py-0">
      <div className="px-5 pt-4 pb-3 border-b border-border flex items-center justify-between gap-2">
        <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Your Position</p>
        {hasFetchError && (
          <button
            type="button"
            onClick={() => setRetryNonce((n) => n + 1)}
            className="text-[10px] font-semibold uppercase tracking-widest text-yellow-700 dark:text-yellow-400 hover:underline transition-colors duration-200"
          >
            Couldn&apos;t load - Retry
          </button>
        )}
      </div>
      {!hasPosition && hasFetchError && (
        <p className="px-5 py-4 text-xs text-muted-foreground">
          Couldn&apos;t load your position - network read failed.
        </p>
      )}
      {hasPosition && position && (
      <div className="px-5 py-4 flex flex-wrap gap-2">
        {position.yesBalance !== "0" && (
          <PositionChip label="YES" value={formatUsdc(BigInt(position.yesBalance))} tone="green" />
        )}
        {position.noBalance !== "0" && (
          <PositionChip label="NO" value={formatUsdc(BigInt(position.noBalance))} tone="red" />
        )}
        {position.lpShares !== "0" && (
          <PositionChip label="LP" value={formatUsdc(BigInt(position.lpShares))} tone="muted" />
        )}
        {position.rootStakeYes !== "0" && (
          <PositionChip label="YES STAKE" value={formatUsdc(BigInt(position.rootStakeYes))} tone="green" />
        )}
        {position.rootStakeNo !== "0" && (
          <PositionChip label="NO STAKE" value={formatUsdc(BigInt(position.rootStakeNo))} tone="red" />
        )}
        {position.deposit !== "0" && (
          <PositionChip label="DEPOSIT" value={formatUsdc(BigInt(position.deposit))} tone="muted" />
        )}
      </div>
      )}
    </Card>
  );
}

function PositionChip({ label, value, tone }: { label: string; value: string; tone: "green" | "red" | "muted" }) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-2 rounded-md border",
        tone === "green" && "bg-green-500/10 border-green-500/20",
        tone === "red" && "bg-red-500/10 border-red-500/20",
        tone === "muted" && "bg-muted/50 border-border"
      )}
    >
      <span
        className={cn(
          "w-1.5 h-1.5 rounded-full shrink-0",
          tone === "green" && "bg-green-500",
          tone === "red" && "bg-red-500",
          tone === "muted" && "bg-muted-foreground/40"
        )}
      />
      <span
        className={cn(
          "text-[10px] font-semibold uppercase tracking-widest",
          tone === "green" && "text-green-700 dark:text-green-400",
          tone === "red" && "text-red-700 dark:text-red-400",
          tone === "muted" && "text-muted-foreground"
        )}
      >
        {label}
      </span>
      <span
        className={cn(
          "text-sm font-semibold font-mono",
          tone === "green" && "text-green-700 dark:text-green-400",
          tone === "red" && "text-red-700 dark:text-red-400",
          tone === "muted" && "text-foreground/80"
        )}
      >
        {value}
      </span>
    </div>
  );
}

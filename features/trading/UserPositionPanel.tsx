"use client";

import { useState, useEffect } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchRawPortfolio } from "@/lib/api/portfolio";
import { formatUsdc } from "@/lib/stellar/scval";

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
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden animate-in fade-in-0 slide-in-from-top-2 duration-300">
      <div className="px-5 pt-4 pb-3 border-b border-white/[0.05] flex items-center justify-between gap-2">
        <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">Your Position</p>
        {hasFetchError && (
          <button
            type="button"
            onClick={() => setRetryNonce((n) => n + 1)}
            className="text-[10px] font-semibold uppercase tracking-widest text-amber-300/80 hover:text-amber-300 transition-colors duration-200"
          >
            Couldn&apos;t load — Retry
          </button>
        )}
      </div>
      {!hasPosition && hasFetchError && (
        <p className="px-5 py-4 text-xs text-white/40">
          Couldn&apos;t load your position — network read failed.
        </p>
      )}
      {hasPosition && position && (
      <div className="px-5 py-4 flex flex-wrap gap-2">
        {position.yesBalance !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-emerald-500/[0.08] border border-emerald-500/[0.15]">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-emerald-400/70">YES</span>
            <span className="text-sm font-semibold font-mono text-emerald-400">
              {formatUsdc(BigInt(position.yesBalance))}
            </span>
          </div>
        )}
        {position.noBalance !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-rose-500/[0.08] border border-rose-500/[0.15]">
            <span className="w-1.5 h-1.5 rounded-full bg-rose-400 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-rose-400/70">NO</span>
            <span className="text-sm font-semibold font-mono text-rose-400">
              {formatUsdc(BigInt(position.noBalance))}
            </span>
          </div>
        )}
        {position.lpShares !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-white/[0.05] border border-white/[0.08]">
            <span className="w-1.5 h-1.5 rounded-full bg-white/40 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-white/40">LP</span>
            <span className="text-sm font-semibold font-mono text-white/70">
              {formatUsdc(BigInt(position.lpShares))}
            </span>
          </div>
        )}
        {position.rootStakeYes !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-emerald-500/[0.08] border border-emerald-500/[0.15]">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-emerald-400/70">YES STAKE</span>
            <span className="text-sm font-semibold font-mono text-emerald-400">
              {formatUsdc(BigInt(position.rootStakeYes))}
            </span>
          </div>
        )}
        {position.rootStakeNo !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-rose-500/[0.08] border border-rose-500/[0.15]">
            <span className="w-1.5 h-1.5 rounded-full bg-rose-400 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-rose-400/70">NO STAKE</span>
            <span className="text-sm font-semibold font-mono text-rose-400">
              {formatUsdc(BigInt(position.rootStakeNo))}
            </span>
          </div>
        )}
        {position.deposit !== "0" && (
          <div className="flex items-center gap-2 px-3 py-2 rounded-xl bg-white/[0.05] border border-white/[0.08]">
            <span className="w-1.5 h-1.5 rounded-full bg-white/40 shrink-0" />
            <span className="text-[10px] font-semibold uppercase tracking-widest text-white/40">DEPOSIT</span>
            <span className="text-sm font-semibold font-mono text-white/70">
              {formatUsdc(BigInt(position.deposit))}
            </span>
          </div>
        )}
      </div>
      )}
    </div>
  );
}

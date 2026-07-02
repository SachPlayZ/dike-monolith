"use client";

import { useState, useEffect } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { ammGetLpBalance, ctBalance } from "@/lib/contracts/clients";
import { formatUsdc } from "@/lib/stellar/scval";

interface PositionData {
  yesBalance: string;
  noBalance: string;
  lpShares: string;
}

interface UserPositionPanelProps {
  marketId: string;
  poolId: string | null;
}

export function UserPositionPanel({ marketId, poolId }: UserPositionPanelProps) {
  const { address, isConnected } = useWallet();
  const [position, setPosition] = useState<PositionData | null>(null);

  useEffect(() => {
    if (!address) return;
    Promise.all([
      ctBalance(address, address, marketId, "Yes").catch(() => "0"),
      ctBalance(address, address, marketId, "No").catch(() => "0"),
      poolId ? ammGetLpBalance(address, address, poolId).catch(() => "0") : Promise.resolve("0"),
    ])
      .then(([yesBalance, noBalance, lpShares]) => {
        setPosition({ yesBalance, noBalance, lpShares });
      })
      .catch(() => {});
  }, [address, marketId, poolId]);

  if (!isConnected || !position) return null;

  const hasPosition =
    position.yesBalance !== "0" || position.noBalance !== "0" || position.lpShares !== "0";
  if (!hasPosition) return null;

  return (
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden animate-in fade-in-0 slide-in-from-top-2 duration-300">
      <div className="px-5 pt-4 pb-3 border-b border-white/[0.05]">
        <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">Your Position</p>
      </div>
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
      </div>
    </div>
  );
}

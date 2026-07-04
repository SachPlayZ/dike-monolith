"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchMarkets } from "@/lib/api/markets";
import { vaultGetAccounting, buildSweepProtocolFees } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState } from "@/lib/types";

interface MarketFees {
  marketId: string;
  question: string;
  protocolFees: string;
  codFees: string;
}

interface SweepProtocolFeesPanelProps {
  // sweep_protocol_fees sweeps the treasury (protocol) + COD shares together in
  // one on-chain call — there's no way to sweep them separately. "admin" shows
  // both amounts and the actionable Sweep button (require_role("gov") enforces
  // the real boundary). "council" is read-only, COD-only — informational for
  // council members who can't sweep but have a legitimate interest in seeing
  // their share accrue.
  variant: "admin" | "council";
}

export function SweepProtocolFeesPanel({ variant }: SweepProtocolFeesPanelProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign, permissions } = useWallet();
  const [expanded, setExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [fees, setFees] = useState<MarketFees[]>([]);
  const [sweepingId, setSweepingId] = useState<string | null>(null);
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const canView = variant === "admin" ? permissions?.canAdmin : permissions?.canCouncil;
  if (!canView) return null;

  async function loadFees() {
    if (!address) return;
    setLoading(true);
    try {
      const markets = await fetchMarkets();
      const rows = await Promise.all(
        markets.map(async (m) => {
          const acc = await vaultGetAccounting(address, m.marketId).catch(() => ({
            protocolFees: "0",
            codFees: "0",
          }));
          return {
            marketId: m.marketId,
            question: m.config.question,
            protocolFees: acc.protocolFees,
            codFees: acc.codFees,
          };
        })
      );
      setFees(
        rows.filter((r) =>
          variant === "admin"
            ? r.protocolFees !== "0" || r.codFees !== "0"
            : r.codFees !== "0"
        )
      );
    } finally {
      setLoading(false);
    }
  }

  function handleExpand() {
    const next = !expanded;
    setExpanded(next);
    if (next) void loadFees();
  }

  async function handleSweep(marketId: string) {
    if (!address) return;
    setSweepingId(marketId);
    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildSweepProtocolFees(address, marketId);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        await loadFees();
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      } finally {
        setSweepingId(null);
      }
    });
  }

  return (
    <div className="rounded-2xl bg-white/[0.03] border border-white/[0.07] overflow-hidden">
      <button
        onClick={handleExpand}
        className="w-full px-5 py-4 flex items-center justify-between border-b border-white/[0.05] hover:bg-white/[0.02] transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-[10px] font-semibold uppercase tracking-[0.15em] text-white/30">
            {variant === "admin" ? "Sweep Protocol Fees" : "COD Fees (read-only)"}
          </p>
          <p className="text-xs text-white/40 mt-0.5">
            {variant === "admin"
              ? "Treasury + COD shares, per market"
              : "Council's share of trading fees — swept by admin along with treasury"}
          </p>
        </div>
        <span className="text-white/30 text-sm">{expanded ? "↑" : "↓"}</span>
      </button>

      {expanded && (
        <div className="p-5 space-y-3">
          {!isConnected ? (
            <button
              onClick={connect}
              className="px-4 py-2 rounded-full bg-orange-500/15 border border-orange-500/25 text-orange-300 text-xs font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-300"
            >
              Connect Wallet
            </button>
          ) : loading ? (
            <p className="text-xs text-white/30 animate-pulse">Checking accrued fees…</p>
          ) : fees.length === 0 ? (
            <p className="text-xs text-white/40">
              {variant === "admin"
                ? "No unswept protocol/COD fees on any market."
                : "No accrued COD fees on any market."}
            </p>
          ) : (
            fees.map((f) => (
              <div
                key={f.marketId}
                className="flex items-center justify-between gap-3 px-4 py-3 rounded-xl bg-white/[0.04] border border-white/[0.07]"
              >
                <div className="min-w-0">
                  <p className="text-xs text-white/70 truncate">{f.question}</p>
                  <p className="text-[10px] text-white/40 font-mono mt-0.5">
                    {variant === "admin"
                      ? `Treasury ${formatUsdc(BigInt(f.protocolFees))} · COD ${formatUsdc(BigInt(f.codFees))}`
                      : `COD ${formatUsdc(BigInt(f.codFees))}`}
                  </p>
                </div>
                {variant === "admin" && (
                  <button
                    onClick={() => handleSweep(f.marketId)}
                    disabled={isPending && sweepingId === f.marketId}
                    className="shrink-0 px-3 py-1.5 rounded-lg bg-orange-500/15 border border-orange-500/25 text-orange-300 text-[10px] font-bold uppercase tracking-widest hover:bg-orange-500/25 transition-all duration-200 disabled:opacity-50"
                  >
                    {isPending && sweepingId === f.marketId ? "Sweeping…" : "Sweep"}
                  </button>
                )}
              </div>
            ))
          )}
          {variant === "admin" && simulatedFee && (
            <p className="text-[10px] text-white/30">Simulated network fee: {simulatedFee}</p>
          )}
          {variant === "admin" && <TxStateDisplay state={txState} />}
        </div>
      )}
    </div>
  );
}

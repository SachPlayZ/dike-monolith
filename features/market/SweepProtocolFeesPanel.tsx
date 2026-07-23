"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/contexts/wallet";
import { fetchMarkets } from "@/lib/api/markets";
import { vaultGetAccounting, buildSweepProtocolFees } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
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
    <Card size="sm" className="overflow-hidden py-0">
      <button
        type="button"
        onClick={handleExpand}
        aria-expanded={expanded}
        aria-controls={`protocol-fees-${variant}`}
        className="w-full px-5 py-4 flex items-center justify-between border-b border-border hover:bg-muted/50 transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">
            {variant === "admin" ? "Sweep Protocol Fees" : "COD Fees (read-only)"}
          </p>
          <p className="text-xs text-muted-foreground/80 mt-0.5">
            {variant === "admin"
              ? "Treasury + COD shares, per market"
              : "Council's share of trading fees - swept by admin along with treasury"}
          </p>
        </div>
        <ChevronDown className={cn("size-4 text-muted-foreground transition-transform duration-200", expanded && "rotate-180")} />
      </button>

      {expanded && (
        <div id={`protocol-fees-${variant}`} className="p-5 space-y-3">
          {!isConnected ? (
            <Button size="sm" onClick={connect}>Connect Wallet</Button>
          ) : loading ? (
            <p className="text-xs text-muted-foreground animate-pulse">Checking accrued fees…</p>
          ) : fees.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {variant === "admin"
                ? "No unswept protocol/COD fees on any market."
                : "No accrued COD fees on any market."}
            </p>
          ) : (
            fees.map((f) => (
              <div
                key={f.marketId}
                className="flex items-center justify-between gap-3 px-4 py-3 rounded-md bg-muted/50 border border-border"
              >
                <div className="min-w-0">
                  <p className="text-xs text-foreground/80 truncate">{f.question}</p>
                  <p className="text-[10px] text-muted-foreground font-mono mt-0.5">
                    {variant === "admin"
                      ? `Treasury ${formatUsdc(BigInt(f.protocolFees))} · COD ${formatUsdc(BigInt(f.codFees))}`
                      : `COD ${formatUsdc(BigInt(f.codFees))}`}
                  </p>
                </div>
                {variant === "admin" && (
                  <Button
                    size="xs"
                    variant="outline"
                    className="shrink-0"
                    onClick={() => handleSweep(f.marketId)}
                    disabled={isPending && sweepingId === f.marketId}
                  >
                    {isPending && sweepingId === f.marketId ? "Sweeping…" : "Sweep"}
                  </Button>
                )}
              </div>
            ))
          )}
          {variant === "admin" && simulatedFee && (
            <p className="text-[10px] text-muted-foreground">Simulated network fee: {simulatedFee}</p>
          )}
          {variant === "admin" && <TxStateDisplay state={txState} />}
        </div>
      )}
    </Card>
  );
}

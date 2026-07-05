"use client";

import { useState, useTransition, useEffect } from "react";
import { useRouter } from "next/navigation";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildAmmAddLiquidity,
  buildAmmRemoveLiquidity,
  buildAmmClaimLpFees,
  ammGetClaimableLpFees,
} from "@/lib/contracts/clients";
import { fetchRawPortfolio } from "@/lib/api/portfolio";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { TxState } from "@/lib/types";

type Mode = "add" | "remove";

interface LiquidityFormProps {
  poolId: string;
  yesReserve: string;
  noReserve: string;
  totalLpShares: string;
}

export function LiquidityForm({ poolId, yesReserve, noReserve, totalLpShares }: LiquidityFormProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign } = useWallet();
  const [expanded, setExpanded] = useState(false);
  const [mode, setMode] = useState<Mode>("add");
  const [amountInput, setAmountInput] = useState("");
  const [lpBalance, setLpBalance] = useState<string | null>(null);
  const [claimableFees, setClaimableFees] = useState<string | null>(null);
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isClaiming, startClaimTransition] = useTransition();
  const [isPending, startTransition] = useTransition();

  function refreshLpState() {
    if (!address) return;
    fetchRawPortfolio(address)
      .then((portfolio) => {
        const lp = portfolio.lpPositions.find((p) => String(p.pool_id) === poolId);
        setLpBalance(String(lp?.shares ?? "0"));
      })
      .catch(() => setLpBalance(null));
    ammGetClaimableLpFees(address, address, poolId)
      .then((fees) => setClaimableFees(fees))
      .catch(() => setClaimableFees(null));
  }

  let expectedLpShares: bigint | null = null;
  if (mode === "add" && amountInput) {
    try {
      const amount = parseUsdc(amountInput);
      const totalShares = BigInt(totalLpShares);
      const yes = BigInt(yesReserve);
      const no = BigInt(noReserve);
      if (amount > 0n && totalShares > 0n && yes > 0n && no > 0n) {
        const yesShares = (totalShares * amount) / yes;
        const noShares = (totalShares * amount) / no;
        expectedLpShares = yesShares < noShares ? yesShares : noShares;
      }
    } catch {
      expectedLpShares = null;
    }
  }

  useEffect(() => {
    if (!address || !expanded) return;
    refreshLpState();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [address, expanded, poolId]);

  async function handleClaimFees() {
    if (!address) return;
    startClaimTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildAmmClaimLpFees(address, poolId);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        refreshLpState();
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  async function handleSubmit() {
    if (!address || !amountInput) return;
    startTransition(async () => {
      try {
        const rawAmount = parseUsdc(amountInput).toString();
        setTxState({ status: "building", hash: null, error: null });
        const xdr =
          mode === "add"
            ? await buildAmmAddLiquidity(address, poolId, rawAmount)
            : await buildAmmRemoveLiquidity(address, poolId, rawAmount);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        refreshLpState();
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-xs text-muted-foreground">Connect wallet to provide liquidity</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card size="sm" className="overflow-hidden py-0">
      {/* Collapsible header */}
      <button
        onClick={() => setExpanded((v) => !v)}
        className="w-full px-5 py-4 flex items-center justify-between border-b border-border hover:bg-muted/50 transition-colors duration-200"
      >
        <div className="text-left">
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Liquidity</p>
          {lpBalance !== null && (
            <p className="text-xs text-foreground/80 mt-0.5 font-mono">
              {formatUsdc(BigInt(lpBalance))} LP
            </p>
          )}
        </div>
        <ChevronDown className={cn("size-4 text-muted-foreground transition-transform duration-200", expanded && "rotate-180")} />
      </button>

      {expanded && (
        <div className="p-5 space-y-4">
          {/* Add / Remove tab */}
          <Tabs value={mode} onValueChange={(v) => { setMode(v as Mode); setAmountInput(""); }}>
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="add">Add</TabsTrigger>
              <TabsTrigger value="remove">Remove</TabsTrigger>
            </TabsList>
          </Tabs>

          {/* LP balance chip (if removing) */}
          {mode === "remove" && lpBalance !== null && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-md bg-muted/50 border border-border">
              <span className="text-xs text-muted-foreground">LP balance</span>
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold font-mono text-foreground/80">
                  {formatUsdc(BigInt(lpBalance))}
                </span>
                <Button
                  size="xs"
                  variant="secondary"
                  onClick={() => setAmountInput(formatUsdc(BigInt(lpBalance)))}
                >
                  Max
                </Button>
              </div>
            </div>
          )}

          {/* Claimable LP fees */}
          {claimableFees !== null && BigInt(claimableFees) > 0n && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-md bg-primary/5 border border-primary/20">
              <div>
                <span className="text-xs text-muted-foreground">Claimable fees</span>
                <p className="text-sm font-semibold font-mono text-primary">
                  {formatUsdc(BigInt(claimableFees))}
                </p>
              </div>
              <Button
                size="xs"
                variant="outline"
                onClick={handleClaimFees}
                disabled={isClaiming || isPending}
              >
                {isClaiming ? "Claiming…" : "Claim"}
              </Button>
            </div>
          )}

          {/* Amount input */}
          <div className="rounded-md border border-input px-4 py-3.5 transition-colors focus-within:border-ring">
            <p className="text-[10px] uppercase tracking-[0.12em] text-muted-foreground mb-2">
              {mode === "add" ? "USDC Amount" : "LP Shares"}
            </p>
            <div className="flex items-baseline gap-2">
              <input
                type="number"
                min="0"
                step="0.01"
                placeholder="0"
                value={amountInput}
                onChange={(e) => setAmountInput(e.target.value)}
                className="flex-1 bg-transparent text-2xl font-semibold text-foreground placeholder:text-muted-foreground/40 outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none w-full min-w-0"
              />
              <span className="text-sm text-muted-foreground shrink-0">
                {mode === "add" ? "USDC" : "LP"}
              </span>
            </div>
          </div>

          {mode === "add" && expectedLpShares !== null && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-md bg-muted/50 border border-border">
              <span className="text-xs text-muted-foreground">Expected LP shares</span>
              <span className="text-sm font-semibold font-mono text-foreground/80">
                {formatUsdc(expectedLpShares)}
              </span>
            </div>
          )}

          {simulatedFee && (
            <div className="flex items-center justify-between px-4 py-2.5 rounded-md bg-muted/50 border border-border">
              <span className="text-xs text-muted-foreground">Simulated network fee</span>
              <span className="text-sm font-semibold font-mono text-foreground/80">
                {simulatedFee}
              </span>
            </div>
          )}

          {/* CTA */}
          <Button
            className="w-full h-11"
            variant={amountInput ? "default" : "outline"}
            onClick={handleSubmit}
            disabled={isPending || isClaiming || !amountInput}
          >
            {isPending
              ? "Processing…"
              : mode === "add"
              ? "Add Liquidity"
              : "Remove Liquidity"}
          </Button>

          <TxStateDisplay state={txState} />
        </div>
      )}
    </Card>
  );
}

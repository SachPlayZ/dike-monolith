"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildAmmAddLiquidity,
  buildAmmRemoveLiquidity,
  ammGetLpBalance,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { TxState } from "@/lib/types";

type Mode = "add" | "remove";

interface LiquidityFormProps {
  poolId: string;
}

export function LiquidityForm({ poolId }: LiquidityFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [mode, setMode] = useState<Mode>("add");
  const [amountInput, setAmountInput] = useState("");
  const [lpBalance, setLpBalance] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  async function loadLpBalance() {
    if (!address) return;
    try {
      const bal = await ammGetLpBalance(address, address, poolId);
      setLpBalance(bal);
    } catch {
      setLpBalance(null);
    }
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

        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);

        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);

        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        await loadLpBalance();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect your wallet to provide liquidity</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card size="sm">
      <CardContent className="space-y-4">
      <h3 className="font-heading text-lg font-normal">Liquidity</h3>

      <div className="flex border border-border overflow-hidden">
        {(["add", "remove"] as Mode[]).map((m) => (
          <Button
            key={m}
            size="xs"
            variant={mode === m ? "default" : "ghost"}
            className="flex-1 capitalize"
            onClick={() => { setMode(m); setAmountInput(""); }}
          >
            {m}
          </Button>
        ))}
      </div>

      {lpBalance !== null && (
        <p className="text-xs text-muted-foreground">
          LP balance: {formatUsdc(BigInt(lpBalance))}
          <Button
            variant="link"
            size="xs"
            className="ml-2"
            onClick={() => setAmountInput(formatUsdc(BigInt(lpBalance)))}
          >
            Max
          </Button>
        </p>
      )}

      <div className="space-y-1">
        <Label className="text-muted-foreground font-medium normal-case tracking-normal">
          {mode === "add" ? "USDC Amount" : "LP Shares"}
        </Label>
        <Input
          type="number"
          min="0"
          step="0.01"
          placeholder="0.00"
          value={amountInput}
          onChange={(e) => setAmountInput(e.target.value)}
        />
      </div>

      <div className="flex gap-2">
        <Button variant="outline" size="xs" onClick={loadLpBalance}>
          Refresh Balance
        </Button>
        <Button
          size="sm"
          className="flex-1"
          onClick={handleSubmit}
          disabled={isPending || !amountInput}
        >
          {isPending
            ? "Processing…"
            : mode === "add"
            ? "Add Liquidity"
            : "Remove Liquidity"}
        </Button>
      </div>

      <TxStateDisplay state={txState} />
      </CardContent>
    </Card>
  );
}

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
      <div className="rounded-lg border border-border p-6 text-center">
        <p className="text-sm text-muted-foreground mb-3">
          Connect your wallet to provide liquidity
        </p>
        <button
          onClick={connect}
          className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          Connect Wallet
        </button>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border p-5 space-y-4">
      <h3 className="text-sm font-semibold">Liquidity</h3>

      <div className="flex rounded-md border border-border overflow-hidden text-sm">
        {(["add", "remove"] as Mode[]).map((m) => (
          <button
            key={m}
            onClick={() => { setMode(m); setAmountInput(""); }}
            className={`flex-1 py-1.5 capitalize transition-colors ${
              mode === m
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted"
            }`}
          >
            {m}
          </button>
        ))}
      </div>

      {lpBalance !== null && (
        <p className="text-xs text-muted-foreground">
          LP balance: {formatUsdc(BigInt(lpBalance))}
          <button
            onClick={() => setAmountInput(formatUsdc(BigInt(lpBalance)))}
            className="ml-2 text-primary underline"
          >
            Max
          </button>
        </p>
      )}

      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          {mode === "add" ? "USDC Amount" : "LP Shares"}
        </label>
        <input
          type="number"
          min="0"
          step="0.01"
          placeholder="0.00"
          value={amountInput}
          onChange={(e) => setAmountInput(e.target.value)}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      <div className="flex gap-2">
        <button
          onClick={loadLpBalance}
          className="rounded-md border border-border px-3 py-2 text-xs hover:bg-muted transition-colors"
        >
          Refresh Balance
        </button>
        <button
          onClick={handleSubmit}
          disabled={isPending || !amountInput}
          className="flex-1 rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          {isPending
            ? "Processing…"
            : mode === "add"
            ? "Add Liquidity"
            : "Remove Liquidity"}
        </button>
      </div>

      <TxStateDisplay state={txState} />
    </div>
  );
}

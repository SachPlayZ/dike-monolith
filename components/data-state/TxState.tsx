"use client";

import type { TxState } from "@/lib/types";
import { LoadingSpinner } from "./LoadingSpinner";

interface TxStateProps {
  state: TxState;
  explorerBase?: string;
}

const STATUS_LABELS: Record<string, string> = {
  idle: "",
  building: "Building transaction…",
  simulating: "Simulating…",
  "awaiting-signature": "Waiting for wallet signature…",
  submitting: "Submitting to network…",
  pending: "Pending confirmation…",
  success: "Transaction confirmed.",
  failed: "Transaction failed.",
};

export function TxStateDisplay({
  state,
  explorerBase = "https://stellar.expert/explorer/testnet/tx",
}: TxStateProps) {
  if (state.status === "idle") return null;

  const isLoading = !["idle", "success", "failed"].includes(state.status);

  return (
    <div
      className={`mt-4 flex flex-col gap-1 rounded-md border px-4 py-3 text-sm ${
        state.status === "success"
          ? "border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-400"
          : state.status === "failed"
          ? "border-destructive/30 bg-destructive/10 text-destructive"
          : "border-border bg-muted/50 text-muted-foreground"
      }`}
    >
      <div className="flex items-center gap-2">
        {isLoading && <LoadingSpinner className="h-4 w-4" />}
        <span>{STATUS_LABELS[state.status] ?? state.status}</span>
      </div>

      {state.hash && (
        <a
          href={`${explorerBase}/${state.hash}`}
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono text-xs underline break-all"
        >
          {state.hash}
        </a>
      )}

      {state.error && (
        <p className="text-xs break-all">{state.error}</p>
      )}
    </div>
  );
}

"use client";

import type { TxState } from "@/lib/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
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

type AlertVariant = "default" | "destructive" | "success" | "warning";

function getVariant(status: string): AlertVariant {
  if (status === "success") return "success";
  if (status === "failed") return "destructive";
  return "default";
}

export function TxStateDisplay({
  state,
  explorerBase = "https://stellar.expert/explorer/testnet/tx",
}: TxStateProps) {
  if (state.status === "idle") return null;

  const isLoading = !["idle", "success", "failed"].includes(state.status);

  return (
    <Alert variant={getVariant(state.status)} className="mt-2">
      <AlertDescription className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          {isLoading && <LoadingSpinner className="h-3.5 w-3.5" />}
          <span>{STATUS_LABELS[state.status] ?? state.status}</span>
        </div>

        {state.hash && (
          <a
            href={`${explorerBase}/${state.hash}`}
            target="_blank"
            rel="noopener noreferrer"
            className="font-mono text-xs underline break-all opacity-80"
          >
            {state.hash}
          </a>
        )}

        {state.error && (
          <p className="text-xs break-all">{state.error}</p>
        )}
      </AlertDescription>
    </Alert>
  );
}

"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  councilCalcCommitment,
  buildCommitVote,
  buildRevealVote,
  buildFinalizeCase,
  buildClaimReward,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { CouncilCase, TxState, Outcome } from "@/lib/types";

interface VoteFormProps {
  councilCase: CouncilCase;
  onSuccess?: () => void;
}

const PENDING_KEY = (address: string, caseId: string) =>
  `dike:council:pending:${address}:${caseId}`;

function genSalt(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

const OUTCOMES: Outcome[] = ["Yes", "No", "Invalid"];

export function VoteForm({ councilCase, onSuccess }: VoteFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [selectedOutcome, setSelectedOutcome] = useState<Outcome>("Yes");
  const [revealSalt, setRevealSalt] = useState("");
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const now = Math.floor(Date.now() / 1000);
  const inCommitPhase =
    councilCase.status === "CommitPhase" && now < councilCase.commitEnd;
  const inRevealPhase =
    councilCase.status === "RevealPhase" &&
    now >= councilCase.commitEnd &&
    now < councilCase.revealEnd;
  const isReadyToFinalize = councilCase.status === "ReadyToFinalize";
  const isFinalized = councilCase.status === "Finalized";

  if (!isConnected) {
    return (
      <div className="rounded-lg border border-border p-4 text-center">
        <p className="text-sm text-muted-foreground mb-3">Connect wallet to vote</p>
        <button onClick={connect} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90">
          Connect Wallet
        </button>
      </div>
    );
  }

  async function exec(buildFn: () => Promise<string>) {
    if (!address) return;
    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildFn();
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        onSuccess?.();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  async function handleCommit() {
    if (!address) return;
    const salt = genSalt();
    const commitment = await councilCalcCommitment(
      address,
      councilCase.caseId,
      address,
      selectedOutcome,
      salt
    );
    // Persist reveal data locally — never sent to server
    const pending = { outcome: selectedOutcome, salt, commitment, caseId: councilCase.caseId };
    localStorage.setItem(PENDING_KEY(address, councilCase.caseId), JSON.stringify(pending));

    await exec(() => buildCommitVote(address, councilCase.caseId, commitment));
    alert(
      `Salt saved locally. Keep it safe — you need it to reveal your vote.\n\nSalt: ${salt}`
    );
  }

  function loadStoredReveal() {
    if (!address) return;
    const stored = localStorage.getItem(PENDING_KEY(address, councilCase.caseId));
    if (!stored) return;
    const pending = JSON.parse(stored) as { outcome: Outcome; salt: string };
    setSelectedOutcome(pending.outcome);
    setRevealSalt(pending.salt);
  }

  return (
    <div className="space-y-4">
      {inCommitPhase && (
        <div className="rounded-lg border border-border p-4 space-y-3">
          <h4 className="text-sm font-semibold">Commit Vote</h4>
          <p className="text-xs text-muted-foreground">
            Commit ends: {new Date(councilCase.commitEnd * 1000).toLocaleString()}
          </p>
          <p className="text-xs text-yellow-700 dark:text-yellow-400 bg-yellow-500/10 rounded p-2">
            Your salt is generated randomly and stored in your browser. Losing it prevents you from revealing your vote.
          </p>
          <div className="flex gap-2">
            {OUTCOMES.map((o) => (
              <button
                key={o}
                onClick={() => setSelectedOutcome(o)}
                className={`flex-1 rounded-md border py-1.5 text-xs transition-colors ${
                  selectedOutcome === o
                    ? "bg-primary border-primary text-primary-foreground"
                    : "border-border text-muted-foreground hover:bg-muted"
                }`}
              >
                {o}
              </button>
            ))}
          </div>
          <button
            onClick={handleCommit}
            disabled={isPending}
            className="w-full rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isPending ? "Processing…" : "Commit Vote"}
          </button>
        </div>
      )}

      {inRevealPhase && (
        <div className="rounded-lg border border-border p-4 space-y-3">
          <h4 className="text-sm font-semibold">Reveal Vote</h4>
          <p className="text-xs text-muted-foreground">
            Reveal ends: {new Date(councilCase.revealEnd * 1000).toLocaleString()}
          </p>
          <button
            onClick={loadStoredReveal}
            className="text-xs text-primary underline"
          >
            Load from browser storage
          </button>
          <div className="flex gap-2">
            {OUTCOMES.map((o) => (
              <button
                key={o}
                onClick={() => setSelectedOutcome(o)}
                className={`flex-1 rounded-md border py-1.5 text-xs transition-colors ${
                  selectedOutcome === o
                    ? "bg-primary border-primary text-primary-foreground"
                    : "border-border text-muted-foreground hover:bg-muted"
                }`}
              >
                {o}
              </button>
            ))}
          </div>
          <div>
            <label className="text-xs text-muted-foreground mb-1 block">Salt (hex)</label>
            <input
              type="text"
              value={revealSalt}
              onChange={(e) => setRevealSalt(e.target.value)}
              placeholder="64-char hex salt"
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-xs font-mono focus:outline-none focus:ring-1 focus:ring-ring"
            />
          </div>
          <button
            onClick={() =>
              exec(() =>
                buildRevealVote(address!, councilCase.caseId, selectedOutcome, revealSalt)
              )
            }
            disabled={isPending || !revealSalt}
            className="w-full rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isPending ? "Processing…" : "Reveal Vote"}
          </button>
        </div>
      )}

      {isReadyToFinalize && (
        <div className="rounded-lg border border-border p-4 space-y-3">
          <h4 className="text-sm font-semibold">Finalize Case</h4>
          <button
            onClick={() => exec(() => buildFinalizeCase(address!, councilCase.caseId))}
            disabled={isPending}
            className="w-full rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isPending ? "Processing…" : "Finalize and Report"}
          </button>
        </div>
      )}

      {isFinalized && (
        <div className="rounded-lg border border-border p-4 space-y-3">
          <h4 className="text-sm font-semibold">Claim Reward</h4>
          <p className="text-xs text-muted-foreground">
            Final outcome: <strong>{councilCase.finalOutcome}</strong>
          </p>
          <button
            onClick={() => exec(() => buildClaimReward(address!, councilCase.caseId))}
            disabled={isPending}
            className="w-full rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isPending ? "Processing…" : "Claim Reward"}
          </button>
        </div>
      )}

      <TxStateDisplay state={txState} />
    </div>
  );
}

"use client";

import { useEffect, useState, useTransition } from "react";
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
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { toast } from "sonner";
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
  const [now, setNow] = useState(0);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  useEffect(() => {
    const updateNow = () => setNow(Math.floor(Date.now() / 1000));
    const timeoutId = window.setTimeout(updateNow, 0);
    const intervalId = window.setInterval(updateNow, 1000);

    return () => {
      window.clearTimeout(timeoutId);
      window.clearInterval(intervalId);
    };
  }, []);

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
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect wallet to vote</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
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
    const pending = { outcome: selectedOutcome, salt, commitment, caseId: councilCase.caseId };
    localStorage.setItem(PENDING_KEY(address, councilCase.caseId), JSON.stringify(pending));

    await exec(() => buildCommitVote(address, councilCase.caseId, commitment));
    toast("Salt saved to browser", {
      description: `Keep it safe — needed to reveal. Salt: ${salt}`,
      duration: 15000,
    });
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
        <Card size="sm">
          <CardContent className="space-y-3">
            <h4 className="font-heading text-lg font-normal">Commit Vote</h4>
            <p className="text-xs text-muted-foreground">
              Commit ends: {new Date(councilCase.commitEnd * 1000).toLocaleString()}
            </p>
            <Alert variant="warning">
              <AlertDescription>
                Salt is generated randomly and stored in your browser. Losing it prevents you from revealing your vote.
              </AlertDescription>
            </Alert>
            <div className="flex gap-2">
              {OUTCOMES.map((o) => (
                <Button
                  key={o}
                  size="xs"
                  variant={selectedOutcome === o ? "default" : "outline"}
                  className="flex-1"
                  onClick={() => setSelectedOutcome(o)}
                >
                  {o}
                </Button>
              ))}
            </div>
            <Button className="w-full" size="sm" onClick={handleCommit} disabled={isPending}>
              {isPending ? "Processing…" : "Commit Vote"}
            </Button>
          </CardContent>
        </Card>
      )}

      {inRevealPhase && (
        <Card size="sm">
          <CardContent className="space-y-3">
            <h4 className="font-heading text-lg font-normal">Reveal Vote</h4>
            <p className="text-xs text-muted-foreground">
              Reveal ends: {new Date(councilCase.revealEnd * 1000).toLocaleString()}
            </p>
            <Button variant="link" size="xs" onClick={loadStoredReveal}>
              Load from browser storage
            </Button>
            <div className="flex gap-2">
              {OUTCOMES.map((o) => (
                <Button
                  key={o}
                  size="xs"
                  variant={selectedOutcome === o ? "default" : "outline"}
                  className="flex-1"
                  onClick={() => setSelectedOutcome(o)}
                >
                  {o}
                </Button>
              ))}
            </div>
            <div className="space-y-1">
              <Label className="text-muted-foreground font-medium normal-case tracking-normal">Salt (hex)</Label>
              <Input
                type="text"
                value={revealSalt}
                onChange={(e) => setRevealSalt(e.target.value)}
                placeholder="64-char hex salt"
                className="font-mono text-xs"
              />
            </div>
            <Button
              className="w-full"
              size="sm"
              onClick={() =>
                exec(() => buildRevealVote(address!, councilCase.caseId, selectedOutcome, revealSalt))
              }
              disabled={isPending || !revealSalt}
            >
              {isPending ? "Processing…" : "Reveal Vote"}
            </Button>
          </CardContent>
        </Card>
      )}

      {isReadyToFinalize && (
        <Card size="sm">
          <CardContent className="space-y-3">
            <h4 className="font-heading text-lg font-normal">Finalize Case</h4>
            <Button
              className="w-full"
              size="sm"
              onClick={() => exec(() => buildFinalizeCase(address!, councilCase.caseId))}
              disabled={isPending}
            >
              {isPending ? "Processing…" : "Finalize and Report"}
            </Button>
          </CardContent>
        </Card>
      )}

      {isFinalized && (
        <Card size="sm">
          <CardContent className="space-y-3">
            <h4 className="font-heading text-lg font-normal">Claim Reward</h4>
            <p className="text-xs text-muted-foreground">
              Final outcome: <strong>{councilCase.finalOutcome}</strong>
            </p>
            <Button
              className="w-full"
              size="sm"
              onClick={() => exec(() => buildClaimReward(address!, councilCase.caseId))}
              disabled={isPending}
            >
              {isPending ? "Processing…" : "Claim Reward"}
            </Button>
          </CardContent>
        </Card>
      )}

      <TxStateDisplay state={txState} />
    </div>
  );
}

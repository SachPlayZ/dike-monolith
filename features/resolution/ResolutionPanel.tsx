"use client";

import { useEffect, useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildRequestResolution,
  buildProposeOutcome,
  buildDisputeOutcome,
  buildFinalizeUndisputed,
  buildEscalateToCouncil,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { MarketData, ResolutionRequest, TxState, Outcome } from "@/lib/types";

interface ResolutionPanelProps {
  market: MarketData;
  request: ResolutionRequest | null;
  onSuccess?: () => void;
}

const OUTCOMES: Outcome[] = ["Yes", "No", "Invalid"];

export function ResolutionPanel({ market, request, onSuccess }: ResolutionPanelProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();
  const [selectedOutcome, setSelectedOutcome] = useState<Outcome>("Yes");
  const [evidenceUri, setEvidenceUri] = useState("");
  const [now, setNow] = useState(0);

  useEffect(() => {
    const updateNow = () => setNow(Math.floor(Date.now() / 1000));
    const timeoutId = window.setTimeout(updateNow, 0);
    const intervalId = window.setInterval(updateNow, 1000);

    return () => {
      window.clearTimeout(timeoutId);
      window.clearInterval(intervalId);
    };
  }, []);

  if (!isConnected) {
    return (
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect wallet to take resolution actions</p>
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

  const marketExpired = now > market.config.expiry;

  return (
    <div className="space-y-5">
      {market.status === "TradingClosed" && !request && marketExpired && (
        <Section title="Request Resolution">
          <p className="text-xs text-muted-foreground mb-3">
            Market has expired. Start the resolution process by requesting COD Oracle.
          </p>
          <Button
            size="sm"
            disabled={isPending}
            onClick={() =>
              exec(() =>
                buildRequestResolution(
                  address!,
                  market.marketId,
                  market.config.questionHash,
                  market.config.rulesUri,
                  market.config.expiry,
                  market.config.bondAmount,
                  market.config.disputeWindow
                )
              )
            }
          >
            Request Resolution
          </Button>
        </Section>
      )}

      {request && request.status === "Requested" && (
        <Section title="Propose Outcome">
          <OutcomeSelector value={selectedOutcome} onChange={setSelectedOutcome} />
          <EvidenceInput value={evidenceUri} onChange={setEvidenceUri} />
          <p className="text-xs text-muted-foreground">
            Bond amount: {market.config.bondAmount} (will be locked until finalization)
          </p>
          <Button
            size="sm"
            disabled={isPending || !evidenceUri}
            onClick={() =>
              exec(() => buildProposeOutcome(address!, request.requestId, selectedOutcome, evidenceUri))
            }
          >
            Propose {selectedOutcome}
          </Button>
        </Section>
      )}

      {request && request.status === "Proposed" && request.proposedAt && (
        <Section title="Dispute Outcome">
          {(() => {
            const disputeDeadline = request.proposedAt + market.config.disputeWindow;
            const canDispute = now < disputeDeadline;
            const canFinalize = !canDispute;
            return (
              <>
                <p className="text-xs text-muted-foreground">
                  Proposed: <strong>{request.proposedOutcome}</strong> by{" "}
                  {request.proposer?.slice(0, 8)}…
                  <br />
                  Dispute window closes:{" "}
                  {new Date(disputeDeadline * 1000).toLocaleString()}
                </p>

                {canDispute && (
                  <>
                    <OutcomeSelector value={selectedOutcome} onChange={setSelectedOutcome} />
                    <EvidenceInput value={evidenceUri} onChange={setEvidenceUri} />
                    <Button
                      size="sm"
                      variant="destructive"
                      disabled={isPending || !evidenceUri}
                      onClick={() =>
                        exec(() =>
                          buildDisputeOutcome(address!, request.requestId, selectedOutcome, evidenceUri)
                        )
                      }
                    >
                      Dispute — Counter {selectedOutcome}
                    </Button>
                  </>
                )}

                {canFinalize && (
                  <Button
                    size="sm"
                    disabled={isPending}
                    onClick={() =>
                      exec(() => buildFinalizeUndisputed(address!, request.requestId))
                    }
                  >
                    Finalize (undisputed)
                  </Button>
                )}
              </>
            );
          })()}
        </Section>
      )}

      {request && request.status === "Disputed" && (
        <Section title="Escalate to Council">
          <p className="text-xs text-muted-foreground mb-3">
            Disputed outcome ready to escalate to Council of Dike for voting.
          </p>
          <Button
            size="sm"
            disabled={isPending}
            onClick={() => exec(() => buildEscalateToCouncil(address!, request.requestId))}
          >
            Escalate to Council
          </Button>
        </Section>
      )}

      <TxStateDisplay state={txState} />
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card size="sm">
      <CardHeader className="pb-0">
        <CardTitle className="text-sm font-semibold normal-case tracking-normal">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">{children}</CardContent>
    </Card>
  );
}

function OutcomeSelector({ value, onChange }: { value: Outcome; onChange: (o: Outcome) => void }) {
  return (
    <div className="flex gap-2">
      {OUTCOMES.map((o) => (
        <Button
          key={o}
          size="xs"
          variant={value === o ? "default" : "outline"}
          className="flex-1"
          onClick={() => onChange(o)}
        >
          {o}
        </Button>
      ))}
    </div>
  );
}

function EvidenceInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <div className="space-y-1">
      <Label className="text-muted-foreground font-medium normal-case tracking-normal">Evidence URI (required)</Label>
      <Input
        type="url"
        placeholder="https://…"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}

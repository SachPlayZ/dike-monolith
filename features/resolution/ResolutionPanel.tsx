"use client";

import { useState, useTransition } from "react";
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

  if (!isConnected) {
    return (
      <div className="rounded-lg border border-border p-6 text-center">
        <p className="text-sm text-muted-foreground mb-3">Connect wallet to take resolution actions</p>
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

  const now = Math.floor(Date.now() / 1000);
  const marketExpired = now > market.config.expiry;

  return (
    <div className="space-y-5">
      {/* Request Resolution */}
      {market.status === "TradingClosed" && !request && marketExpired && (
        <Section title="Request Resolution">
          <p className="text-xs text-muted-foreground mb-3">
            Market has expired. Start the resolution process by requesting COD Oracle.
          </p>
          <button
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
            disabled={isPending}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            Request Resolution
          </button>
        </Section>
      )}

      {/* Propose Outcome */}
      {request && request.status === "Requested" && (
        <Section title="Propose Outcome">
          <OutcomeSelector value={selectedOutcome} onChange={setSelectedOutcome} />
          <EvidenceInput value={evidenceUri} onChange={setEvidenceUri} />
          <p className="text-xs text-muted-foreground">
            Bond amount: {market.config.bondAmount} (will be locked until finalization)
          </p>
          <button
            onClick={() =>
              exec(() =>
                buildProposeOutcome(
                  address!,
                  request.requestId,
                  selectedOutcome,
                  evidenceUri
                )
              )
            }
            disabled={isPending || !evidenceUri}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            Propose {selectedOutcome}
          </button>
        </Section>
      )}

      {/* Dispute */}
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
                    <button
                      onClick={() =>
                        exec(() =>
                          buildDisputeOutcome(
                            address!,
                            request.requestId,
                            selectedOutcome,
                            evidenceUri
                          )
                        )
                      }
                      disabled={isPending || !evidenceUri}
                      className="rounded-md border border-destructive px-4 py-2 text-sm text-destructive hover:bg-destructive/10 disabled:opacity-50"
                    >
                      Dispute — Counter {selectedOutcome}
                    </button>
                  </>
                )}

                {canFinalize && (
                  <button
                    onClick={() =>
                      exec(() => buildFinalizeUndisputed(address!, request.requestId))
                    }
                    disabled={isPending}
                    className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                  >
                    Finalize (undisputed)
                  </button>
                )}
              </>
            );
          })()}
        </Section>
      )}

      {/* Escalate to Council */}
      {request && request.status === "Disputed" && (
        <Section title="Escalate to Council">
          <p className="text-xs text-muted-foreground mb-3">
            Disputed outcome ready to escalate to Council of Dike for voting.
          </p>
          <button
            onClick={() =>
              exec(() => buildEscalateToCouncil(address!, request.requestId))
            }
            disabled={isPending}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            Escalate to Council
          </button>
        </Section>
      )}

      <TxStateDisplay state={txState} />
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-border p-4 space-y-3">
      <h4 className="text-sm font-semibold">{title}</h4>
      {children}
    </div>
  );
}

function OutcomeSelector({
  value,
  onChange,
}: {
  value: Outcome;
  onChange: (o: Outcome) => void;
}) {
  return (
    <div className="flex gap-2">
      {OUTCOMES.map((o) => (
        <button
          key={o}
          onClick={() => onChange(o)}
          className={`flex-1 rounded-md border py-1.5 text-xs transition-colors ${
            value === o
              ? "bg-primary border-primary text-primary-foreground"
              : "border-border text-muted-foreground hover:bg-muted"
          }`}
        >
          {o}
        </button>
      ))}
    </div>
  );
}

function EvidenceInput({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div>
      <label className="text-xs text-muted-foreground mb-1 block">
        Evidence URI (required)
      </label>
      <input
        type="url"
        placeholder="https://…"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
      />
    </div>
  );
}

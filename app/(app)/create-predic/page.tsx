"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { buildCreateMarket, type CreateMarketParams } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { COLLATERAL_CONTRACT } from "@/lib/contracts/manifest";
import type { TxState } from "@/lib/types";

// Opening price is fixed at 5000 bps — contract rejects any other value
const OPENING_PRICE_BPS = 5000;

export default function CreateMarketPage() {
  const { address, isConnected, connect, sign } = useWallet();
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const [form, setForm] = useState({
    question: "",
    questionHash: "",
    rulesUri: "",
    rulesHash: "",
    category: "",
    expiry: "",
    collateral: COLLATERAL_CONTRACT,
    bondAmount: "10",
    disputeWindowHours: "48",
    initialLiquidity: "100",
  });

  function set(field: keyof typeof form) {
    return (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
      setForm((f) => ({ ...f, [field]: e.target.value }));
  }

  function validate(): string | null {
    if (!form.question.trim()) return "Question is required";
    if (!form.rulesUri.trim()) return "Rules URI is required";
    if (!form.expiry) return "Expiry is required";
    const expiryTs = Math.floor(new Date(form.expiry).getTime() / 1000);
    if (expiryTs <= Math.floor(Date.now() / 1000)) return "Expiry must be in the future";
    if (!form.collateral.trim()) return "Collateral address is required";
    if (Number(form.bondAmount) <= 0) return "Bond amount must be positive";
    if (Number(form.disputeWindowHours) <= 0) return "Dispute window must be positive";
    if (Number(form.initialLiquidity) <= 0) return "Initial liquidity must be positive";
    return null;
  }

  async function handleCreate() {
    if (!address) return;
    const err = validate();
    if (err) {
      setTxState({ status: "failed", hash: null, error: err });
      return;
    }

    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });

        const expiryTs = Math.floor(new Date(form.expiry).getTime() / 1000);
        const params: CreateMarketParams = {
          question: form.question,
          questionHash: form.questionHash || "0".repeat(64),
          rulesUri: form.rulesUri,
          rulesHash: form.rulesHash || "0".repeat(64),
          category: form.category || "General",
          expiry: expiryTs,
          collateral: form.collateral,
          bondAmount: parseUsdc(form.bondAmount).toString(),
          disputeWindow: Number(form.disputeWindowHours) * 3600,
        };
        const initialLiquidity = parseUsdc(form.initialLiquidity).toString();

        const xdr = await buildCreateMarket(address, params, initialLiquidity, OPENING_PRICE_BPS);

        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);

        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);

        setTxState({ status: "success", hash: result.hash, error: null });
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold tracking-tight">Create Market</h1>
        <div className="rounded-lg border border-border p-6 text-center">
          <p className="text-sm text-muted-foreground mb-3">
            Connect your wallet. Only approved creators can create markets.
          </p>
          <button
            onClick={connect}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
          >
            Connect Wallet
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Create Market</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Only approved creators can create markets. Opening price is fixed at 50/50.
        </p>
      </div>

      <div className="rounded-lg border border-border p-6 space-y-5">
        <Field label="Question *">
          <textarea
            value={form.question}
            onChange={set("question")}
            rows={2}
            className="input"
            placeholder="Will X happen by date Y?"
          />
        </Field>

        <Field label="Question Hash (bytes32 hex)">
          <input type="text" value={form.questionHash} onChange={set("questionHash")} className="input" placeholder="0x…" />
        </Field>

        <Field label="Rules URI *">
          <input type="url" value={form.rulesUri} onChange={set("rulesUri")} className="input" placeholder="https://…" />
        </Field>

        <Field label="Rules Hash (bytes32 hex)">
          <input type="text" value={form.rulesHash} onChange={set("rulesHash")} className="input" placeholder="0x…" />
        </Field>

        <Field label="Category">
          <input type="text" value={form.category} onChange={set("category")} className="input" placeholder="Politics, Sports, Crypto…" />
        </Field>

        <Field label="Expiry *">
          <input type="datetime-local" value={form.expiry} onChange={set("expiry")} className="input" />
        </Field>

        <Field label="Collateral Address">
          <input type="text" value={form.collateral} onChange={set("collateral")} className="input font-mono text-xs" />
        </Field>

        <div className="grid grid-cols-3 gap-4">
          <Field label="Bond (USDC)">
            <input type="number" min="0" step="0.01" value={form.bondAmount} onChange={set("bondAmount")} className="input" />
          </Field>
          <Field label="Dispute window (hours)">
            <input type="number" min="1" value={form.disputeWindowHours} onChange={set("disputeWindowHours")} className="input" />
          </Field>
          <Field label="Initial liquidity (USDC)">
            <input type="number" min="0" step="0.01" value={form.initialLiquidity} onChange={set("initialLiquidity")} className="input" />
          </Field>
        </div>

        <div className="rounded-md bg-muted/50 px-4 py-3 text-xs text-muted-foreground">
          Opening price: <strong>50 / 50</strong> (fixed — contract enforces this)
        </div>

        <button
          onClick={handleCreate}
          disabled={isPending}
          className="w-full rounded-md bg-primary py-2.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          {isPending ? "Processing…" : "Create Market"}
        </button>

        <TxStateDisplay state={txState} />
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <label className="text-xs font-medium text-muted-foreground">{label}</label>
      {children}
    </div>
  );
}

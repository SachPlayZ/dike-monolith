"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { buildCreateMarket, type CreateMarketParams } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
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
        <h1 className="font-heading text-3xl font-normal tracking-tight">Create Market</h1>
        <Card>
          <CardContent className="p-6 text-center space-y-3">
            <p className="text-sm text-muted-foreground">
              Connect your wallet. Only approved creators can create markets.
            </p>
            <Button size="sm" onClick={connect}>Connect Wallet</Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">Create Market</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Only approved creators can create markets. Opening price is fixed at 50/50.
        </p>
      </div>

      <Card>
        <CardContent className="p-6 space-y-5">
        <Field label="Question *">
          <Textarea
            value={form.question}
            onChange={set("question")}
            rows={2}
            placeholder="Will X happen by date Y?"
          />
        </Field>

        <Field label="Question Hash (bytes32 hex)">
          <Input type="text" value={form.questionHash} onChange={set("questionHash")} placeholder="0x…" />
        </Field>

        <Field label="Rules URI *">
          <Input type="url" value={form.rulesUri} onChange={set("rulesUri")} placeholder="https://…" />
        </Field>

        <Field label="Rules Hash (bytes32 hex)">
          <Input type="text" value={form.rulesHash} onChange={set("rulesHash")} placeholder="0x…" />
        </Field>

        <Field label="Category">
          <Input type="text" value={form.category} onChange={set("category")} placeholder="Politics, Sports, Crypto…" />
        </Field>

        <Field label="Expiry *">
          <Input type="datetime-local" value={form.expiry} onChange={set("expiry")} />
        </Field>

        <Field label="Collateral Address">
          <Input type="text" value={form.collateral} onChange={set("collateral")} className="font-mono text-xs" />
        </Field>

        <div className="grid grid-cols-3 gap-4">
          <Field label="Bond (USDC)">
            <Input type="number" min="0" step="0.01" value={form.bondAmount} onChange={set("bondAmount")} />
          </Field>
          <Field label="Dispute window (hours)">
            <Input type="number" min="1" value={form.disputeWindowHours} onChange={set("disputeWindowHours")} />
          </Field>
          <Field label="Initial liquidity (USDC)">
            <Input type="number" min="0" step="0.01" value={form.initialLiquidity} onChange={set("initialLiquidity")} />
          </Field>
        </div>

        <div className="bg-muted/50 px-4 py-3 text-xs text-muted-foreground">
          Opening price: <strong>50 / 50</strong> (fixed — contract enforces this)
        </div>

        <Button
          onClick={handleCreate}
          disabled={isPending}
          className="w-full"
          size="sm"
        >
          {isPending ? "Processing…" : "Create Market"}
        </Button>

        <TxStateDisplay state={txState} />
        </CardContent>
      </Card>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <Label className="text-muted-foreground font-medium normal-case tracking-normal">{label}</Label>
      {children}
    </div>
  );
}

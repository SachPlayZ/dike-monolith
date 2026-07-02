"use client";

import { useEffect, useState, useTransition } from "react";
import * as StellarSdk from "@stellar/stellar-sdk";
import { format } from "date-fns";
import { CalendarIcon } from "lucide-react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildCreateMarket,
  feeManagerGetConfig,
  type CreateMarketParams,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { EmptyState } from "@/components/data-state/EmptyState";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { COLLATERAL_CONTRACT } from "@/lib/contracts/manifest";
import { MARKET_CATEGORIES, type TxState } from "@/lib/types";
import { networkConfig } from "@/lib/stellar/config";
import { toast } from "sonner";

// Opening price is fixed at 5000 bps — contract rejects any other value
const OPENING_PRICE_BPS = 5000;

async function sha256Hex(value: string) {
  const bytes = new TextEncoder().encode(value.trim());
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function parseExpiryValue(value: string) {
  return value ? new Date(value) : undefined;
}

function toExpiryValue(date: Date | undefined, time: string) {
  if (!date) return "";
  const [hours = "23", minutes = "59"] = time.split(":");
  const next = new Date(date);
  next.setHours(Number(hours), Number(minutes), 0, 0);
  const timezoneOffset = next.getTimezoneOffset() * 60_000;
  return new Date(next.getTime() - timezoneOffset).toISOString().slice(0, 16);
}

export default function CreateMarketPage() {
  const {
    address,
    isConnected,
    connect,
    sign,
    permissions,
    permissionsLoading,
    isConnecting,
  } = useWallet();
  const [txState, setTxState] = useState<TxState>({
    status: "idle",
    hash: null,
    error: null,
  });
  const [isPending, startTransition] = useTransition();
  const [creationFee, setCreationFee] = useState<string | null>(null);

  useEffect(() => {
    if (!address) return;
    feeManagerGetConfig(address)
      .then((cfg) => setCreationFee(cfg.creationFee))
      .catch(() => setCreationFee(null));
  }, [address]);

  const [form, setForm] = useState({
    question: "",
    rulesUri: "",
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
    if (!form.category) return "Category is required";
    if (!form.expiry) return "Expiry is required";
    const expiryTs = Math.floor(new Date(form.expiry).getTime() / 1000);
    if (expiryTs <= Math.floor(Date.now() / 1000))
      return "Expiry must be in the future";
    if (!form.collateral.trim()) return "Collateral address is required";
    try {
      StellarSdk.Address.fromString(form.collateral.trim());
    } catch {
      return "Collateral address must be a valid Stellar address";
    }
    if (Number(form.bondAmount) <= 0) return "Bond amount must be positive";
    if (Number(form.disputeWindowHours) <= 0)
      return "Dispute window must be positive";
    if (Number(form.initialLiquidity) <= 0)
      return "Initial liquidity must be positive";
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
        const [questionHash, rulesHash] = await Promise.all([
          sha256Hex(form.question),
          sha256Hex(form.rulesUri),
        ]);
        const params: CreateMarketParams = {
          question: form.question,
          questionHash,
          rulesUri: form.rulesUri,
          rulesHash,
          category: form.category,
          expiry: expiryTs,
          collateral: form.collateral,
          bondAmount: parseUsdc(form.bondAmount).toString(),
          disputeWindow: Number(form.disputeWindowHours) * 3600,
        };
        const initialLiquidity = parseUsdc(form.initialLiquidity).toString();

        const xdr = await buildCreateMarket(
          address,
          params,
          initialLiquidity,
          OPENING_PRICE_BPS,
        );

        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);

        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);

        const explorerUrl = `https://stellar.expert/explorer/${networkConfig.network}/tx/${result.hash}`;
        toast.success("Market Created", {
          description: "Transaction confirmed on Stellar.",
          action: {
            label: "Open Stellar Expert",
            onClick: () =>
              window.open(explorerUrl, "_blank", "noopener,noreferrer"),
          },
          duration: 12000,
        });

        setTxState({ status: "idle", hash: null, error: null });
        setForm({
          question: "",
          rulesUri: "",
          category: "",
          expiry: "",
          collateral: COLLATERAL_CONTRACT,
          bondAmount: "10",
          disputeWindowHours: "48",
          initialLiquidity: "100",
        });
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="space-y-6">
        <h1 className="font-heading text-3xl font-normal tracking-tight">
          Create Market
        </h1>
        <EmptyState
          title="Connect your wallet"
          description="Only approved creators can create markets."
          action={
            <Button size="sm" onClick={connect} disabled={isConnecting}>
              {isConnecting ? "Connecting…" : "Connect Wallet"}
            </Button>
          }
        />
      </div>
    );
  }

  if (permissionsLoading) {
    return (
      <div className="space-y-6">
        <h1 className="font-heading text-3xl font-normal tracking-tight">
          Create Market
        </h1>
        <Card>
          <CardContent className="p-6 text-sm text-muted-foreground">
            Verifying approved creator access…
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!permissions?.canCreate) {
    return (
      <div className="space-y-6">
        <h1 className="font-heading text-3xl font-normal tracking-tight">
          Create Market
        </h1>
        <EmptyState
          title="Creator access required"
          description="Connected wallet is not approved to create markets."
        />
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">
          Create Market
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Only approved creators can create markets. Opening price is fixed at
          50/50.
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

          <Field label="Rules URI *">
            <Input
              type="url"
              value={form.rulesUri}
              onChange={set("rulesUri")}
              placeholder="https://…"
            />
          </Field>

          <Field label="Category *">
            <div className="flex flex-wrap gap-2">
              {MARKET_CATEGORIES.map((category) => (
                <button
                  key={category}
                  type="button"
                  onClick={() =>
                    setForm((current) => ({ ...current, category }))
                  }
                  className={cn(
                    "rounded-full border px-3 py-1 text-xs font-medium uppercase tracking-wide transition-colors",
                    form.category === category
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-border text-muted-foreground hover:border-primary/50 hover:text-foreground"
                  )}
                >
                  {category}
                </button>
              ))}
            </div>
          </Field>

          <Field label="Expiry *">
            <ExpiryPicker
              value={form.expiry}
              onChange={(value) =>
                setForm((current) => ({ ...current, expiry: value }))
              }
            />
          </Field>

          <Field label="Collateral Address">
            <Input
              type="text"
              value={form.collateral}
              onChange={set("collateral")}
              className="font-mono text-xs"
            />
          </Field>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <Field label="Bond (USDC)">
              <Input
                type="number"
                min="0"
                step="0.01"
                value={form.bondAmount}
                onChange={set("bondAmount")}
              />
            </Field>
            <Field label="Dispute window (hours)">
              <Input
                type="number"
                min="1"
                value={form.disputeWindowHours}
                onChange={set("disputeWindowHours")}
              />
            </Field>
            <Field label="Initial liquidity (USDC)">
              <Input
                type="number"
                min="0"
                step="0.01"
                value={form.initialLiquidity}
                onChange={set("initialLiquidity")}
              />
            </Field>
          </div>

          <div className="bg-muted/50 px-4 py-3 text-xs text-muted-foreground">
            Question hash and rules hash are auto-generated from your inputs.
            <br />
            Opening price: <strong>50 / 50</strong> (fixed — contract enforces
            this)
            {creationFee !== null && BigInt(creationFee) > 0n && (
              <>
                <br />
                Creation fee:{" "}
                <strong>{formatUsdc(BigInt(creationFee))} USDC</strong> (charged
                to treasury in your chosen collateral on submit)
              </>
            )}
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

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-muted-foreground font-medium normal-case tracking-normal">
        {label}
      </Label>
      {children}
    </div>
  );
}

function ExpiryPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const selectedDate = parseExpiryValue(value);
  const currentTime = value ? value.slice(11, 16) : "23:59";

  return (
    <div className="space-y-2">
      <Popover>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            className={cn(
              "w-full justify-between rounded-none border-x-0 border-t-0 px-0 text-left font-normal hover:bg-transparent",
              !selectedDate && "text-muted-foreground",
            )}
          >
            <span>
              {selectedDate
                ? format(selectedDate, "PPP p")
                : "Pick expiry date and time"}
            </span>
            <CalendarIcon className="size-4 opacity-70" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-auto p-0">
          <PopoverHeader className="px-4 pt-4">
            <PopoverTitle>Expiry</PopoverTitle>
            <PopoverDescription>
              Choose market close date, then set exact local time.
            </PopoverDescription>
          </PopoverHeader>
          <div className="border-t border-border/60">
            <Calendar
              mode="single"
              selected={selectedDate}
              onSelect={(date) => onChange(toExpiryValue(date, currentTime))}
              disabled={(date) =>
                date < new Date(new Date().setHours(0, 0, 0, 0))
              }
              className="mx-auto"
            />
          </div>
          <div className="border-t border-border/60 p-4">
            <Label className="mb-2 block text-xs text-muted-foreground font-medium normal-case tracking-normal">
              Time
            </Label>
            <Input
              type="time"
              value={currentTime}
              onChange={(event) =>
                onChange(
                  toExpiryValue(selectedDate ?? new Date(), event.target.value),
                )
              }
            />
          </div>
        </PopoverContent>
      </Popover>
      <p className="text-xs text-muted-foreground">
        Stored using your local timezone, then converted before submit.
      </p>
    </div>
  );
}

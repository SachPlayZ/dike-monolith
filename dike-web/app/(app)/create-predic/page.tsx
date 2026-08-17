"use client";

import * as React from "react";
import { useEffect, useRef, useState, useTransition } from "react";
import * as StellarSdk from "@stellar/stellar-sdk";
import { CalendarIcon } from "lucide-react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildCreateMarket,
  feeManagerGetConfig,
  type CreateMarketParams,
} from "@/lib/contracts/clients";
import { parseDikeError } from "@/lib/stellar/transaction";
import { executeTransaction } from "@/lib/stellar/execute";
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
import { getReferenceUrlError } from "@/lib/validation/reference-url";
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
  const [fieldErrors, setFieldErrors] = useState<Partial<Record<keyof typeof form, string>>>({});
  const fieldRefs = useRef<Partial<Record<keyof typeof form, HTMLElement | null>>>({});

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

  function validate(): { field: keyof typeof form; message: string } | null {
    if (!form.question.trim()) return { field: "question", message: "Question is required" };
    const rulesUrlError = getReferenceUrlError(form.rulesUri, "Rules URL");
    if (rulesUrlError) return { field: "rulesUri", message: rulesUrlError };
    if (!form.category) return { field: "category", message: "Category is required" };
    if (!form.expiry) return { field: "expiry", message: "Expiry is required" };
    const expiryTs = Math.floor(new Date(form.expiry).getTime() / 1000);
    if (expiryTs <= Math.floor(Date.now() / 1000))
      return { field: "expiry", message: "Expiry must be in the future" };
    if (!form.collateral.trim()) return { field: "collateral", message: "Collateral address is required" };
    try {
      StellarSdk.Address.fromString(form.collateral.trim());
    } catch {
      return { field: "collateral", message: "Collateral address must be a valid Stellar address" };
    }
    if (Number(form.bondAmount) <= 0) return { field: "bondAmount", message: "Bond amount must be positive" };
    if (Number(form.disputeWindowHours) <= 0)
      return { field: "disputeWindowHours", message: "Dispute window must be positive" };
    if (Number(form.initialLiquidity) <= 0)
      return { field: "initialLiquidity", message: "Initial liquidity must be positive" };
    return null;
  }

  async function handleCreate() {
    if (!address) return;
    const err = validate();
    if (err) {
      setFieldErrors({ [err.field]: err.message });
      fieldRefs.current[err.field]?.focus();
      return;
    }
    setFieldErrors({});

    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });

        const expiryTs = Math.floor(new Date(form.expiry).getTime() / 1000);
        const [questionHash, rulesHash] = await Promise.all([
          sha256Hex(form.question.trim()),
          sha256Hex(form.rulesUri.trim()),
        ]);
        const params: CreateMarketParams = {
          question: form.question.trim(),
          questionHash,
          rulesUri: form.rulesUri.trim(),
          rulesHash,
          category: form.category,
          expiry: expiryTs,
          collateral: form.collateral,
          bondAmount: parseUsdc(form.bondAmount).toString(),
          disputeWindow: Number(form.disputeWindowHours) * 3600,
        };
        const initialLiquidity = parseUsdc(form.initialLiquidity).toString();

        const result = await executeTransaction({
          build: () => buildCreateMarket(address, params, initialLiquidity, OPENING_PRICE_BPS),
          sign,
          method: "create_market",
          onState: setTxState,
        });

        const explorerUrl = `https://stellar.expert/explorer/${networkConfig.explorerNetwork}/tx/${result.hash}`;
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
          <Field label="Question *" htmlFor="question" error={fieldErrors.question}>
            <Textarea
              id="question"
              name="question"
              autoComplete="off"
              ref={(el) => { fieldRefs.current.question = el; }}
              aria-invalid={!!fieldErrors.question}
              value={form.question}
              onChange={set("question")}
              rows={2}
              placeholder="Will X happen by date Y?"
            />
          </Field>

          <Field label="Rules URI *" htmlFor="rulesUri" error={fieldErrors.rulesUri}>
            <Input
              id="rulesUri"
              name="rulesUri"
              type="url"
              autoComplete="off"
              ref={(el) => { fieldRefs.current.rulesUri = el; }}
              aria-invalid={!!fieldErrors.rulesUri}
              value={form.rulesUri}
              onChange={set("rulesUri")}
              placeholder="https://…"
              aria-describedby="rulesUri-help"
            />
            <p id="rulesUri-help" className="text-xs text-muted-foreground">
              Permanent public HTTPS page with the resolution source and criteria. Placeholder URLs are rejected.
            </p>
          </Field>

          <Field label="Category *" error={fieldErrors.category}>
            <div
              className="flex flex-wrap gap-2"
              ref={(el) => { fieldRefs.current.category = el; }}
              tabIndex={-1}
            >
              {MARKET_CATEGORIES.map((category) => (
                <button
                  key={category}
                  type="button"
                  aria-pressed={form.category === category}
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

          <Field label="Expiry *" error={fieldErrors.expiry}>
            <ExpiryPicker
              ref={(el) => { fieldRefs.current.expiry = el; }}
              value={form.expiry}
              onChange={(value) =>
                setForm((current) => ({ ...current, expiry: value }))
              }
            />
          </Field>

          <Field label="Collateral Address" htmlFor="collateral" error={fieldErrors.collateral}>
            <Input
              id="collateral"
              name="collateral"
              type="text"
              autoComplete="off"
              spellCheck={false}
              ref={(el) => { fieldRefs.current.collateral = el; }}
              aria-invalid={!!fieldErrors.collateral}
              value={form.collateral}
              onChange={set("collateral")}
              className="font-mono text-xs"
            />
          </Field>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <Field label="Bond (USDC)" htmlFor="bondAmount" error={fieldErrors.bondAmount}>
              <Input
                id="bondAmount"
                name="bondAmount"
                type="number"
                autoComplete="off"
                inputMode="decimal"
                ref={(el) => { fieldRefs.current.bondAmount = el; }}
                aria-invalid={!!fieldErrors.bondAmount}
                min="0"
                step="0.01"
                value={form.bondAmount}
                onChange={set("bondAmount")}
              />
            </Field>
            <Field label="Dispute window (hours)" htmlFor="disputeWindowHours" error={fieldErrors.disputeWindowHours}>
              <Input
                id="disputeWindowHours"
                name="disputeWindowHours"
                type="number"
                autoComplete="off"
                inputMode="numeric"
                ref={(el) => { fieldRefs.current.disputeWindowHours = el; }}
                aria-invalid={!!fieldErrors.disputeWindowHours}
                min="1"
                value={form.disputeWindowHours}
                onChange={set("disputeWindowHours")}
              />
            </Field>
            <Field label="Initial liquidity (USDC)" htmlFor="initialLiquidity" error={fieldErrors.initialLiquidity}>
              <Input
                id="initialLiquidity"
                name="initialLiquidity"
                type="number"
                autoComplete="off"
                inputMode="decimal"
                ref={(el) => { fieldRefs.current.initialLiquidity = el; }}
                aria-invalid={!!fieldErrors.initialLiquidity}
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
  htmlFor,
  error,
  children,
}: {
  label: string;
  htmlFor?: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <Label
        htmlFor={htmlFor}
        className="text-muted-foreground font-medium normal-case tracking-normal"
      >
        {label}
      </Label>
      {children}
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

const ExpiryPicker = React.forwardRef<HTMLButtonElement, {
  value: string;
  onChange: (value: string) => void;
}>(function ExpiryPicker({ value, onChange }, ref) {
  const selectedDate = parseExpiryValue(value);
  const currentTime = value ? value.slice(11, 16) : "23:59";

  return (
    <div className="space-y-2">
      <Popover>
        <PopoverTrigger asChild>
          <Button
            ref={ref}
            type="button"
            variant="outline"
            className={cn(
              "w-full justify-between rounded-none border-x-0 border-t-0 px-0 text-left font-normal hover:bg-transparent",
              !selectedDate && "text-muted-foreground",
            )}
          >
            <span>
              {selectedDate
                ? new Intl.DateTimeFormat(undefined, {
                    dateStyle: "long",
                    timeStyle: "short",
                  }).format(selectedDate)
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
            <Label
              htmlFor="expiry-time"
              className="mb-2 block text-xs text-muted-foreground font-medium normal-case tracking-normal"
            >
              Time
            </Label>
            <Input
              id="expiry-time"
              name="expiry-time"
              autoComplete="off"
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
});

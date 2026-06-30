"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  ammQuoteBuyYes,
  ammQuoteBuyNo,
  buildAmmBuyChildYes,
  buildAmmBuyChildNo,
  vaultGetChildAvail,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import type { TxState, Outcome } from "@/lib/types";

interface ChildTradeFormProps {
  poolId: string;
}

const DEFAULT_SLIPPAGE_BPS = 50;
const TRADE_DEADLINE_SECS = 300;

type ChildToken = "yes" | "no";

export function ChildTradeForm({ poolId }: ChildTradeFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [parentMarketId, setParentMarketId] = useState("");
  const [parentOutcome, setParentOutcome] = useState<"Yes" | "No">("Yes");
  const [availableCredit, setAvailableCredit] = useState<string | null>(null);
  const [creditLoading, setCreditLoading] = useState(false);
  const [token, setToken] = useState<ChildToken>("yes");
  const [amountInput, setAmountInput] = useState("");
  const [quote, setQuote] = useState<{ amountOut: string; feeBps: number } | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  async function handleCheckCredit() {
    if (!address || !parentMarketId.trim()) return;
    setCreditLoading(true);
    setAvailableCredit(null);
    try {
      const avail = await vaultGetChildAvail(address, address, parentMarketId.trim(), parentOutcome as Outcome);
      setAvailableCredit(avail);
    } catch (e) {
      setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
    } finally {
      setCreditLoading(false);
    }
  }

  async function handleQuote() {
    if (!address || !amountInput) return;
    setQuoting(true);
    try {
      const rawIn = parseUsdc(amountInput).toString();
      const q =
        token === "yes"
          ? await ammQuoteBuyYes(address, poolId, rawIn)
          : await ammQuoteBuyNo(address, poolId, rawIn);
      setQuote({ amountOut: q.amountOut, feeBps: q.feeBps });
    } catch (e) {
      setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
    } finally {
      setQuoting(false);
    }
  }

  async function handleBuy() {
    if (!address || !amountInput || !quote || !parentMarketId.trim()) return;
    startTransition(async () => {
      try {
        const rawIn = parseUsdc(amountInput).toString();
        const minOut = applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS).toString();

        setTxState({ status: "building", hash: null, error: null });

        const xdr =
          token === "yes"
            ? await buildAmmBuyChildYes(
                address,
                parentMarketId.trim(),
                parentOutcome as Outcome,
                poolId,
                rawIn,
                minOut,
                TRADE_DEADLINE_SECS
              )
            : await buildAmmBuyChildNo(
                address,
                parentMarketId.trim(),
                parentOutcome as Outcome,
                poolId,
                rawIn,
                minOut,
                TRADE_DEADLINE_SECS
              );

        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);

        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);

        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        setQuote(null);
        setAvailableCredit(null);
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <div className="rounded-lg border border-border p-6 text-center">
        <p className="text-sm text-muted-foreground mb-3">
          Connect wallet to buy with parent credit
        </p>
        <button
          onClick={connect}
          className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          Connect Wallet
        </button>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border p-5 space-y-4">
      <div>
        <h3 className="text-sm font-semibold">Buy with Parent Credit</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Use your stake in a parent market as collateral. Max 60% of root stake.
        </p>
      </div>

      <div className="rounded-md bg-amber-500/10 border border-amber-500/30 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
        Child credit encumbers your parent position. Selling or redeeming the parent requires clearing child debt first.
      </div>

      {/* Parent market inputs */}
      <div className="space-y-3">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Parent Market ID</label>
          <input
            type="text"
            placeholder="e.g. 1"
            value={parentMarketId}
            onChange={(e) => { setParentMarketId(e.target.value); setAvailableCredit(null); setQuote(null); }}
            className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>

        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Parent Outcome to Encumber</label>
          <div className="flex rounded-md border border-border overflow-hidden text-sm">
            {(["Yes", "No"] as const).map((o) => (
              <button
                key={o}
                onClick={() => { setParentOutcome(o); setAvailableCredit(null); setQuote(null); }}
                className={`flex-1 py-1.5 transition-colors ${
                  parentOutcome === o
                    ? o === "Yes"
                      ? "bg-green-600 text-white"
                      : "bg-red-600 text-white"
                    : "text-muted-foreground hover:bg-muted"
                }`}
              >
                {o}
              </button>
            ))}
          </div>
        </div>

        <button
          onClick={handleCheckCredit}
          disabled={creditLoading || !parentMarketId.trim()}
          className="w-full rounded-md border border-border py-1.5 text-xs hover:bg-muted transition-colors disabled:opacity-50"
        >
          {creditLoading ? "Checking…" : "Check Available Credit"}
        </button>

        {availableCredit !== null && (
          <div className="rounded-md bg-muted/50 px-3 py-2 text-xs">
            <span className="text-muted-foreground">Available credit: </span>
            <span className="font-medium">{formatUsdc(BigInt(availableCredit))} USDC</span>
            {availableCredit === "0" && (
              <p className="text-amber-600 dark:text-amber-400 mt-1">
                No credit available. You need a root stake in this parent market outcome.
              </p>
            )}
          </div>
        )}
      </div>

      {/* Token toggle */}
      <div>
        <label className="text-xs text-muted-foreground mb-1 block">Buy Token</label>
        <div className="flex rounded-md border border-border overflow-hidden text-sm">
          {(["yes", "no"] as ChildToken[]).map((t) => (
            <button
              key={t}
              onClick={() => { setToken(t); setQuote(null); }}
              className={`flex-1 py-1.5 uppercase transition-colors ${
                token === t
                  ? t === "yes"
                    ? "bg-green-600 text-white"
                    : "bg-red-600 text-white"
                  : "text-muted-foreground hover:bg-muted"
              }`}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {/* Amount */}
      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          Amount (credit units)
        </label>
        <input
          type="number"
          min="0"
          step="0.01"
          placeholder="0.00"
          value={amountInput}
          onChange={(e) => { setAmountInput(e.target.value); setQuote(null); }}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      {/* Quote display */}
      {quote && (
        <div className="rounded-md bg-muted/50 p-3 space-y-1 text-xs">
          <p className="flex justify-between">
            <span className="text-muted-foreground">Estimated out</span>
            <span>{formatUsdc(BigInt(quote.amountOut))} tokens</span>
          </p>
          <p className="flex justify-between">
            <span className="text-muted-foreground">Fee</span>
            <span>{quote.feeBps / 100}%</span>
          </p>
          <p className="flex justify-between">
            <span className="text-muted-foreground">Slippage tolerance</span>
            <span>{DEFAULT_SLIPPAGE_BPS / 100}%</span>
          </p>
          <p className="flex justify-between">
            <span className="text-muted-foreground">Min received</span>
            <span>
              {formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} tokens
            </span>
          </p>
          <p className="flex justify-between">
            <span className="text-muted-foreground">Deadline</span>
            <span>{TRADE_DEADLINE_SECS / 60} min</span>
          </p>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2">
        <button
          onClick={handleQuote}
          disabled={quoting || !amountInput}
          className="flex-1 rounded-md border border-border py-2 text-sm hover:bg-muted transition-colors disabled:opacity-50"
        >
          {quoting ? "Quoting…" : "Get Quote"}
        </button>
        <button
          onClick={handleBuy}
          disabled={isPending || !quote || !amountInput || !parentMarketId.trim()}
          className="flex-1 rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          {isPending ? "Processing…" : `Buy ${token.toUpperCase()}`}
        </button>
      </div>

      <TxStateDisplay state={txState} />
    </div>
  );
}

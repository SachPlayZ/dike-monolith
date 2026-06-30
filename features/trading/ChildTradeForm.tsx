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
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect wallet to buy with parent credit</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card size="sm">
      <CardContent className="space-y-4">
      <div>
        <h3 className="font-heading text-lg font-normal">Buy with Parent Credit</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Use your stake in a parent market as collateral. Max 60% of root stake.
        </p>
      </div>

      <div className="bg-amber-500/10 border border-amber-500/30 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
        Child credit encumbers your parent position. Selling or redeeming the parent requires clearing child debt first.
      </div>

      <div className="space-y-3">
        <div className="space-y-1">
          <Label className="text-muted-foreground font-medium normal-case tracking-normal">Parent Market ID</Label>
          <Input
            type="text"
            placeholder="e.g. 1"
            value={parentMarketId}
            onChange={(e) => { setParentMarketId(e.target.value); setAvailableCredit(null); setQuote(null); }}
          />
        </div>

        <div>
          <label className="text-xs text-muted-foreground mb-1 block">Parent Outcome to Encumber</label>
          <div className="flex border border-border overflow-hidden">
            {(["Yes", "No"] as const).map((o) => (
              <Button
                key={o}
                size="xs"
                className={`flex-1 ${
                  parentOutcome === o
                    ? o === "Yes"
                      ? "bg-green-600 hover:bg-green-700 border-green-600 text-white"
                      : "bg-red-600 hover:bg-red-700 border-red-600 text-white"
                    : ""
                }`}
                variant={parentOutcome === o ? "default" : "ghost"}
                onClick={() => { setParentOutcome(o); setAvailableCredit(null); setQuote(null); }}
              >
                {o}
              </Button>
            ))}
          </div>
        </div>

        <Button
          variant="outline"
          size="xs"
          className="w-full"
          onClick={handleCheckCredit}
          disabled={creditLoading || !parentMarketId.trim()}
        >
          {creditLoading ? "Checking…" : "Check Available Credit"}
        </Button>

        {availableCredit !== null && (
          <div className="bg-muted/50 px-3 py-2 text-xs">
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

      <div>
        <label className="text-xs text-muted-foreground mb-1 block">Buy Token</label>
        <div className="flex border border-border overflow-hidden">
          {(["yes", "no"] as ChildToken[]).map((t) => (
            <Button
              key={t}
              size="xs"
              className={`flex-1 uppercase ${
                token === t
                  ? t === "yes"
                    ? "bg-green-600 hover:bg-green-700 border-green-600 text-white"
                    : "bg-red-600 hover:bg-red-700 border-red-600 text-white"
                  : ""
              }`}
              variant={token === t ? "default" : "ghost"}
              onClick={() => { setToken(t); setQuote(null); }}
            >
              {t}
            </Button>
          ))}
        </div>
      </div>

      <div className="space-y-1">
        <Label className="text-muted-foreground font-medium normal-case tracking-normal">Amount (credit units)</Label>
        <Input
          type="number"
          min="0"
          step="0.01"
          placeholder="0.00"
          value={amountInput}
          onChange={(e) => { setAmountInput(e.target.value); setQuote(null); }}
        />
      </div>

      {quote && (
        <div className="bg-muted/50 p-3 space-y-1 text-xs">
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
            <span>{formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} tokens</span>
          </p>
          <p className="flex justify-between">
            <span className="text-muted-foreground">Deadline</span>
            <span>{TRADE_DEADLINE_SECS / 60} min</span>
          </p>
        </div>
      )}

      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={handleQuote}
          disabled={quoting || !amountInput}
        >
          {quoting ? "Quoting…" : "Get Quote"}
        </Button>
        <Button
          size="sm"
          className="flex-1"
          onClick={handleBuy}
          disabled={isPending || !quote || !amountInput || !parentMarketId.trim()}
        >
          {isPending ? "Processing…" : `Buy ${token.toUpperCase()}`}
        </Button>
      </div>

      <TxStateDisplay state={txState} />
      </CardContent>
    </Card>
  );
}

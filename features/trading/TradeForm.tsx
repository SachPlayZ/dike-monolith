"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  ammQuoteBuyYes,
  ammQuoteBuyNo,
  buildAmmBuyYes,
  buildAmmBuyNo,
  buildAmmSellYes,
  buildAmmSellNo,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { parseUsdc, formatUsdc, applySlippage } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { TxState } from "@/lib/types";

type Side = "buy" | "sell";
type Token = "yes" | "no";

interface TradeFormProps {
  marketId: string;
  poolId: string;
  marketQuestion: string;
}

const DEFAULT_SLIPPAGE_BPS = 50;
const TRADE_DEADLINE_SECS = 300;

export function TradeForm({ marketId, poolId, marketQuestion }: TradeFormProps) {
  const { address, isConnected, connect, sign } = useWallet();
  const [side, setSide] = useState<Side>("buy");
  const [token, setToken] = useState<Token>("yes");
  const [amountInput, setAmountInput] = useState("");
  const [quote, setQuote] = useState<{ amountOut: string; feeBps: number } | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  async function handleQuote() {
    if (!address || !amountInput) return;
    setQuoting(true);
    try {
      const rawIn = parseUsdc(amountInput).toString();
      let q;
      if (side === "buy") {
        q = token === "yes"
          ? await ammQuoteBuyYes(address, poolId, rawIn)
          : await ammQuoteBuyNo(address, poolId, rawIn);
      } else {
        q = token === "yes"
          ? await ammQuoteBuyNo(address, poolId, rawIn)
          : await ammQuoteBuyYes(address, poolId, rawIn);
      }
      setQuote({ amountOut: q.amountOut, feeBps: q.feeBps });
    } catch (e) {
      setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
    } finally {
      setQuoting(false);
    }
  }

  async function handleTrade() {
    if (!address || !amountInput || !quote) return;
    startTransition(async () => {
      try {
        const rawIn = parseUsdc(amountInput).toString();
        const minOut = applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS).toString();

        setTxState({ status: "building", hash: null, error: null });

        let xdr: string;
        if (side === "buy") {
          xdr = token === "yes"
            ? await buildAmmBuyYes(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS)
            : await buildAmmBuyNo(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS);
        } else {
          xdr = token === "yes"
            ? await buildAmmSellYes(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS)
            : await buildAmmSellNo(address, poolId, rawIn, minOut, TRADE_DEADLINE_SECS);
        }

        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);

        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);

        setTxState({ status: "success", hash: result.hash, error: null });
        setAmountInput("");
        setQuote(null);
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  if (!isConnected) {
    return (
      <Card size="sm">
        <CardContent className="text-center space-y-3">
          <p className="text-sm text-muted-foreground">Connect your wallet to trade</p>
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card size="sm">
      <CardContent className="space-y-4">
      <h3 className="font-heading text-lg font-normal">Trade</h3>

      <div className="flex border border-border overflow-hidden">
        {(["buy", "sell"] as Side[]).map((s) => (
          <Button
            key={s}
            size="xs"
            variant={side === s ? "default" : "ghost"}
            className="flex-1 capitalize"
            onClick={() => { setSide(s); setQuote(null); }}
          >
            {s}
          </Button>
        ))}
      </div>

      <div className="flex border border-border overflow-hidden">
        {(["yes", "no"] as Token[]).map((t) => (
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

      <div className="space-y-1">
        <Label className="text-muted-foreground font-medium normal-case tracking-normal">Amount (USDC)</Label>
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
            <span>{formatUsdc(BigInt(quote.amountOut))} USDC</span>
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
            <span>{formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} USDC</span>
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
          onClick={handleTrade}
          disabled={isPending || !quote || !amountInput}
        >
          {isPending
            ? "Processing…"
            : `${side === "buy" ? "Buy" : "Sell"} ${token.toUpperCase()}`}
        </Button>
      </div>

      <TxStateDisplay state={txState} />
      </CardContent>
    </Card>
  );
}

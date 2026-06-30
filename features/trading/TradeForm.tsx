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
import type { TxState } from "@/lib/types";

type Side = "buy" | "sell";
type Token = "yes" | "no";

interface TradeFormProps {
  marketId: string;
  poolId: string;
  marketQuestion: string;
}

const DEFAULT_SLIPPAGE_BPS = 50; // 0.5%
const TRADE_DEADLINE_SECS = 300; // 5 min

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
        // For sell, quote is just the inverse direction
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
        const minOut = applySlippage(
          BigInt(quote.amountOut),
          DEFAULT_SLIPPAGE_BPS
        ).toString();

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
      <div className="rounded-lg border border-border p-6 text-center">
        <p className="text-sm text-muted-foreground mb-3">
          Connect your wallet to trade
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
      <h3 className="text-sm font-semibold">Trade</h3>

      {/* Side toggle */}
      <div className="flex rounded-md border border-border overflow-hidden text-sm">
        {(["buy", "sell"] as Side[]).map((s) => (
          <button
            key={s}
            onClick={() => { setSide(s); setQuote(null); }}
            className={`flex-1 py-1.5 capitalize transition-colors ${
              side === s
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted"
            }`}
          >
            {s}
          </button>
        ))}
      </div>

      {/* Token toggle */}
      <div className="flex rounded-md border border-border overflow-hidden text-sm">
        {(["yes", "no"] as Token[]).map((t) => (
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

      {/* Amount input */}
      <div>
        <label className="text-xs text-muted-foreground mb-1 block">
          Amount (USDC)
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

      {/* Quote */}
      {quote && (
        <div className="rounded-md bg-muted/50 p-3 space-y-1 text-xs">
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
            <span>
              {formatUsdc(applySlippage(BigInt(quote.amountOut), DEFAULT_SLIPPAGE_BPS))} USDC
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
          onClick={handleTrade}
          disabled={isPending || !quote || !amountInput}
          className="flex-1 rounded-md bg-primary py-2 text-sm text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          {isPending
            ? "Processing…"
            : `${side === "buy" ? "Buy" : "Sell"} ${token.toUpperCase()}`}
        </button>
      </div>

      <TxStateDisplay state={txState} />
    </div>
  );
}

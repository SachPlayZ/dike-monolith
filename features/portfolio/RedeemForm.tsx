"use client";

import { useState, useTransition } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import {
  buildRedeemResolved,
  buildRedeemCancelled,
} from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError } from "@/lib/stellar/transaction";
import { formatUsdc } from "@/lib/stellar/scval";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Button } from "@/components/ui/button";
import type { UserPosition, TxState, Outcome } from "@/lib/types";

interface RedeemFormProps {
  position: UserPosition;
  onSuccess?: () => void;
}

export function RedeemForm({ position, onSuccess }: RedeemFormProps) {
  const { address, sign } = useWallet();
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const isResolved = position.marketStatus === "Resolved";
  const isCancelled = position.marketStatus === "Cancelled";

  if (!isResolved && !isCancelled) {
    return (
      <p className="text-xs text-muted-foreground">
        Market must be Resolved or Cancelled to redeem.
      </p>
    );
  }

  async function handleRedeem(outcome: Outcome, balance: string) {
    if (!address) return;
    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });

        const xdr = isResolved
          ? await buildRedeemResolved(address, position.marketId, outcome, balance)
          : await buildRedeemCancelled(address, position.marketId, outcome, balance);

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

  const yesBalance = BigInt(position.yesBalance);
  const noBalance = BigInt(position.noBalance);

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        {isResolved
          ? `Redeem winning tokens. Final outcome: ${position.finalOutcome}.`
          : "Redeem at cancelled-market payout."}
      </p>

      <div className="flex gap-2">
        {yesBalance > 0n && (
          <Button
            size="sm"
            className="flex-1 bg-green-600 hover:bg-green-700 border-green-600 text-white"
            onClick={() => handleRedeem("Yes", yesBalance.toString())}
            disabled={isPending}
          >
            Redeem YES ({formatUsdc(yesBalance)} USDC)
          </Button>
        )}
        {noBalance > 0n && (
          <Button
            size="sm"
            className="flex-1 bg-red-600 hover:bg-red-700 border-red-600 text-white"
            onClick={() => handleRedeem("No", noBalance.toString())}
            disabled={isPending}
          >
            Redeem NO ({formatUsdc(noBalance)} USDC)
          </Button>
        )}
      </div>

      <TxStateDisplay state={txState} />
    </div>
  );
}

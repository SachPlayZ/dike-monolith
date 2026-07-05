"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { useWallet } from "@/lib/contexts/wallet";
import { buildCloseTrading } from "@/lib/contracts/clients";
import { submitAndPoll, parseDikeError, feeFromXdr, formatFeeXlm } from "@/lib/stellar/transaction";
import { TxStateDisplay } from "@/components/data-state/TxState";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import type { TxState } from "@/lib/types";

interface CloseTradingButtonProps {
  marketId: string;
  expiry: number;
}

// Contract-side close_trading is permissionless (no require_auth) — any wallet
// can call it once expiry passes. Gated to admins here purely as a UI choice
// to keep the action out of regular traders' way.
export function CloseTradingButton({ marketId, expiry }: CloseTradingButtonProps) {
  const router = useRouter();
  const { address, isConnected, connect, sign, permissions } = useWallet();
  const [simulatedFee, setSimulatedFee] = useState<string | null>(null);
  const [txState, setTxState] = useState<TxState>({ status: "idle", hash: null, error: null });
  const [isPending, startTransition] = useTransition();

  const isExpired = Date.now() / 1000 >= expiry;
  if (!isExpired || !permissions?.canAdmin) return null;

  async function handleClose() {
    if (!address) return;
    startTransition(async () => {
      try {
        setTxState({ status: "building", hash: null, error: null });
        const xdr = await buildCloseTrading(address, marketId);
        setSimulatedFee(formatFeeXlm(feeFromXdr(xdr)));
        setTxState({ status: "awaiting-signature", hash: null, error: null });
        const signedXdr = await sign(xdr);
        setTxState({ status: "submitting", hash: null, error: null });
        const result = await submitAndPoll(signedXdr);
        setTxState({ status: "success", hash: result.hash, error: null });
        router.refresh();
      } catch (e) {
        setTxState({ status: "failed", hash: null, error: parseDikeError(e) });
      }
    });
  }

  return (
    <Card size="sm">
      <CardContent className="space-y-3">
        <Alert variant="warning">
          <AlertDescription>
            <span className="font-semibold text-yellow-700 dark:text-yellow-400">Trading window expired.</span>{" "}
            Expiry passed but on-chain status is still Live - no one has closed trading yet.
          </AlertDescription>
        </Alert>
        {isConnected ? (
          <Button size="sm" onClick={handleClose} disabled={isPending}>
            {isPending ? "Closing…" : "Close Trading"}
          </Button>
        ) : (
          <Button size="sm" onClick={connect}>Connect Wallet</Button>
        )}
        {simulatedFee && (
          <p className="text-[10px] text-muted-foreground">Simulated network fee: {simulatedFee}</p>
        )}
        <TxStateDisplay state={txState} />
      </CardContent>
    </Card>
  );
}

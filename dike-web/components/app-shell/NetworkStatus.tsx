"use client";

import { useWallet } from "@/lib/contexts/wallet";
import { configurationErrors, networkConfig } from "@/lib/stellar/config";
import { Alert, AlertDescription } from "@/components/ui/alert";

export function NetworkStatus() {
  const { networkError } = useWallet();
  const messages = [...configurationErrors, ...(networkError ? [networkError] : [])];

  if (messages.length === 0) return null;

  return (
    <div className="fixed inset-x-3 top-3 z-[100] mx-auto max-w-3xl" role="status" aria-live="polite">
      <Alert variant="destructive" className="shadow-xl">
        <AlertDescription>
          <span className="font-semibold">{networkConfig.label} configuration blocked.</span>{" "}
          {messages.join(" ")}
        </AlertDescription>
      </Alert>
    </div>
  );
}

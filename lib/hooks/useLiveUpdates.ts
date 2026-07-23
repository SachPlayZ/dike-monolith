"use client";

import { useEffect, useRef } from "react";
import { NETWORK } from "@/lib/stellar/config";

export type StateUpdate =
  | { type: "market"; network: string; marketId: number }
  | { type: "portfolio"; network: string; address: string }
  | { type: "governance"; network: string }
  | { type: "council_case"; network: string; caseId: number }
  | { type: "timelock_action"; network: string; actionId: number };

// Subscribes to dike-services' SSE push channel (proxied through /api/stream
// to keep the connection same-origin, same reason apiGet routes client-side
// reads through /api/proxy). Each ledger's writes arrive here as they land,
// instead of waiting out a client-side poll interval.
export function useLiveUpdates(onUpdate: (update: StateUpdate) => void) {
  const handlerRef = useRef(onUpdate);

  useEffect(() => {
    handlerRef.current = onUpdate;
  }, [onUpdate]);

  useEffect(() => {
    const source = new EventSource("/api/stream");

    const listener = (event: MessageEvent<string>) => {
      try {
        const update = JSON.parse(event.data) as StateUpdate;
        if (update.network === NETWORK) handlerRef.current(update);
      } catch {
        // malformed payload; ignore rather than tear down the connection
      }
    };

    source.addEventListener("update", listener);

    return () => {
      source.removeEventListener("update", listener);
      source.close();
    };
  }, []);
}

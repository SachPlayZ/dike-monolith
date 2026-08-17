"use client";

import { useRef } from "react";
import { useRouter } from "next/navigation";
import { useLiveUpdates, type StateUpdate } from "@/lib/hooks/useLiveUpdates";

interface LiveRefreshProps {
  /** Only re-fetch when the pushed update is about this market (governance updates always refresh). */
  marketId?: number;
}

const DEBOUNCE_MS = 300;

export function LiveRefresh({ marketId }: LiveRefreshProps) {
  const router = useRouter();
  const pendingRef = useRef(false);

  useLiveUpdates((update: StateUpdate) => {
    const relevant =
      marketId === undefined ||
      update.type === "governance" ||
      (update.type === "market" && update.marketId === marketId);
    if (!relevant || pendingRef.current) return;

    pendingRef.current = true;
    setTimeout(() => {
      pendingRef.current = false;
      router.refresh();
    }, DEBOUNCE_MS);
  });

  return null;
}

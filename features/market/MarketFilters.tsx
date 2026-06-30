"use client";

import { useRouter, useSearchParams } from "next/navigation";
import type { MarketStatus } from "@/lib/types";

const STATUSES: MarketStatus[] = [
  "Live",
  "Proposed",
  "Disputed",
  "CouncilVoting",
  "Resolved",
  "Cancelled",
];

export function MarketFilters() {
  const router = useRouter();
  const params = useSearchParams();
  const current = params.get("status") ?? "";

  function setFilter(status: string) {
    const next = new URLSearchParams(params.toString());
    if (status) next.set("status", status);
    else next.delete("status");
    router.push(`?${next.toString()}`);
  }

  return (
    <div className="flex flex-wrap gap-2">
      <button
        onClick={() => setFilter("")}
        className={`rounded-full px-3 py-1 text-xs border transition-colors ${
          !current
            ? "bg-primary text-primary-foreground border-primary"
            : "border-border text-muted-foreground hover:border-foreground"
        }`}
      >
        All
      </button>
      {STATUSES.map((s) => (
        <button
          key={s}
          onClick={() => setFilter(s)}
          className={`rounded-full px-3 py-1 text-xs border transition-colors ${
            current === s
              ? "bg-primary text-primary-foreground border-primary"
              : "border-border text-muted-foreground hover:border-foreground"
          }`}
        >
          {s}
        </button>
      ))}
    </div>
  );
}

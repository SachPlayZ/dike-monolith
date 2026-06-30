"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/button";
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
      <Button
        size="xs"
        variant={!current ? "default" : "outline"}
        className="rounded-full"
        onClick={() => setFilter("")}
      >
        All
      </Button>
      {STATUSES.map((s) => (
        <Button
          key={s}
          size="xs"
          variant={current === s ? "default" : "outline"}
          className="rounded-full"
          onClick={() => setFilter(s)}
        >
          {s}
        </Button>
      ))}
    </div>
  );
}

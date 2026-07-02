"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { MARKET_CATEGORIES, type MarketStatus } from "@/lib/types";

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
  const currentCategory = params.get("category") ?? "";

  function setFilter(status: string) {
    const next = new URLSearchParams(params.toString());
    if (status) next.set("status", status);
    else next.delete("status");
    router.push(`?${next.toString()}`);
  }

  function setCategory(category: string) {
    const next = new URLSearchParams(params.toString());
    if (category) next.set("category", category);
    else next.delete("category");
    router.push(`?${next.toString()}`);
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        <Button
          size="xs"
          variant={!current ? "default" : "outline"}
          className="rounded-full"
          onClick={() => setFilter("")}
        >
          All Statuses
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
      <div className="flex flex-wrap gap-2">
        <Button
          size="xs"
          variant={!currentCategory ? "default" : "outline"}
          className="rounded-full"
          onClick={() => setCategory("")}
        >
          All Categories
        </Button>
        {MARKET_CATEGORIES.map((category) => (
          <Button
            key={category}
            size="xs"
            variant={currentCategory === category ? "default" : "outline"}
            className="rounded-full"
            onClick={() => setCategory(category)}
          >
            {category}
          </Button>
        ))}
      </div>
    </div>
  );
}

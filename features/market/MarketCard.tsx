import Link from "next/link";
import type { MarketData } from "@/lib/types";
import { MarketStatusBadge } from "./MarketStatusBadge";
import { formatUsdc, impliedYesBps } from "@/lib/stellar/scval";

interface MarketCardProps {
  market: MarketData;
}

function ImpliedPrice({ yesBps }: { yesBps: number }) {
  const yesPct = (yesBps / 100).toFixed(1);
  const noPct = ((10000 - yesBps) / 100).toFixed(1);
  return (
    <div className="flex gap-3 text-xs">
      <span className="text-green-600 dark:text-green-400">
        YES {yesPct}¢
      </span>
      <span className="text-red-600 dark:text-red-400">
        NO {noPct}¢
      </span>
    </div>
  );
}

export function MarketCard({ market }: MarketCardProps) {
  const expiry = new Date(market.config.expiry * 1000);
  const yesBps = impliedYesBps(market.yesReserve, market.noReserve);
  const isTradeable = market.status === "Live";

  return (
    <Link
      href={`/markets/${market.marketId}`}
      className="block rounded-lg border border-border bg-card p-4 hover:bg-muted/50 transition-colors"
    >
      <div className="flex items-start justify-between gap-2 mb-2">
        <p className="text-sm font-medium leading-snug line-clamp-2">
          {market.config.question}
        </p>
        <MarketStatusBadge status={market.status} />
      </div>

      <div className="flex items-center gap-3 mt-3">
        {isTradeable && <ImpliedPrice yesBps={yesBps} />}
        {market.finalOutcome && (
          <span
            className={`text-xs font-semibold ${
              market.finalOutcome === "Yes"
                ? "text-green-600"
                : market.finalOutcome === "No"
                ? "text-red-600"
                : "text-muted-foreground"
            }`}
          >
            Resolved: {market.finalOutcome}
          </span>
        )}
        <span className="ml-auto text-xs text-muted-foreground">
          {market.config.category}
        </span>
      </div>

      <p className="mt-1 text-xs text-muted-foreground">
        Expires{" "}
        {expiry.toLocaleDateString(undefined, {
          month: "short",
          day: "numeric",
          year: "numeric",
        })}
      </p>
    </Link>
  );
}

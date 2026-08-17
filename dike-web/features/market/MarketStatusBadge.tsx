import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import type { MarketStatus } from "@/lib/types";

const STATUS_COLORS: Record<MarketStatus, string> = {
  Created: "bg-muted text-muted-foreground",
  Live: "bg-green-500/20 text-green-700 dark:text-green-400",
  Paused: "bg-yellow-500/20 text-yellow-700 dark:text-yellow-400",
  TradingClosed: "bg-orange-500/20 text-orange-700 dark:text-orange-400",
  ResolutionRequested: "bg-blue-500/20 text-blue-700 dark:text-blue-400",
  Proposed: "bg-purple-500/20 text-purple-700 dark:text-purple-400",
  Disputed: "bg-red-500/20 text-red-700 dark:text-red-400",
  CouncilVoting: "bg-indigo-500/20 text-indigo-700 dark:text-indigo-400",
  Resolved: "bg-emerald-500/20 text-emerald-700 dark:text-emerald-400",
  Cancelled: "bg-muted text-muted-foreground line-through",
};

export function MarketStatusBadge({ status }: { status: MarketStatus }) {
  return (
    <Badge
      className={cn(
        "rounded-full px-2 py-0.5 text-xs font-medium normal-case tracking-normal",
        STATUS_COLORS[status]
      )}
    >
      {status}
    </Badge>
  );
}

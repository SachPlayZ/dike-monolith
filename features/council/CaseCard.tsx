import type { CouncilCase } from "@/lib/types";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";

const STATUS_COLORS: Record<string, string> = {
  Opened: "bg-muted text-muted-foreground",
  CommitPhase: "bg-blue-500/20 text-blue-700 dark:text-blue-400",
  RevealPhase: "bg-purple-500/20 text-purple-700 dark:text-purple-400",
  ReadyToFinalize: "bg-yellow-500/20 text-yellow-700 dark:text-yellow-400",
  Finalized: "bg-green-500/20 text-green-700 dark:text-green-400",
  Cancelled: "bg-muted text-muted-foreground",
};

interface CaseCardProps {
  councilCase: CouncilCase;
  expanded?: boolean;
  onClick?: () => void;
}

export function CaseCard({ councilCase, expanded, onClick }: CaseCardProps) {
  return (
    <Card
      size="sm"
      className={cn(onClick ? "cursor-pointer hover:bg-muted/50 transition-colors" : "")}
      onClick={onClick}
    >
      <CardContent className="space-y-3">
        <div className="flex items-center justify-between">
          <p className="text-xs font-mono text-muted-foreground">
            Case #{councilCase.caseId.slice(0, 8)}…
          </p>
          <Badge
            className={cn(
              "rounded-full px-2 py-0.5 text-xs font-medium normal-case tracking-normal",
              STATUS_COLORS[councilCase.status] ?? ""
            )}
          >
            {councilCase.status}
          </Badge>
        </div>

        <div className="grid grid-cols-2 gap-2 text-xs">
          <Kv label="Proposed" value={councilCase.proposedOutcome} />
          <Kv label="Disputed" value={councilCase.disputedOutcome} />
          <Kv
            label="Commit end"
            value={new Date(councilCase.commitEnd * 1000).toLocaleString()}
          />
          <Kv
            label="Reveal end"
            value={new Date(councilCase.revealEnd * 1000).toLocaleString()}
          />
          {councilCase.finalOutcome && (
            <Kv label="Final outcome" value={councilCase.finalOutcome} />
          )}
        </div>

        {councilCase.evidenceUri && (
          <a
            href={councilCase.evidenceUri}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-primary underline break-all"
            onClick={(e) => e.stopPropagation()}
          >
            Evidence
          </a>
        )}
      </CardContent>
    </Card>
  );
}

function Kv({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-muted-foreground">{label}</p>
      <p className="font-medium">{value}</p>
    </div>
  );
}

import type { UserPosition } from "@/lib/types";
import { MarketStatusBadge } from "@/features/market/MarketStatusBadge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { formatUsdc } from "@/lib/stellar/scval";

interface PositionCardProps {
  position: UserPosition;
  onRedeem?: (position: UserPosition) => void;
}

export function PositionCard({ position, onRedeem }: PositionCardProps) {
  const hasYes = BigInt(position.yesBalance) > 0n;
  const hasNo = BigInt(position.noBalance) > 0n;
  const hasLp = BigInt(position.lpShares) > 0n;
  const isRedeemable =
    (position.marketStatus === "Resolved" ||
      position.marketStatus === "Cancelled") &&
    (hasYes || hasNo);

  return (
    <Card size="sm">
      <CardContent className="space-y-3">
        <div className="flex items-start justify-between gap-2">
          <p className="text-sm font-medium leading-snug line-clamp-2">
            {position.question}
          </p>
          <MarketStatusBadge status={position.marketStatus} />
        </div>

        <div className="grid grid-cols-2 gap-2 text-xs">
          {hasYes && (
            <Stat label="YES tokens" value={formatUsdc(BigInt(position.yesBalance))} />
          )}
          {hasNo && (
            <Stat label="NO tokens" value={formatUsdc(BigInt(position.noBalance))} />
          )}
          {hasLp && (
            <Stat label="LP shares" value={formatUsdc(BigInt(position.lpShares))} />
          )}
          {BigInt(position.deposit) > 0n && (
            <Stat label="Deposit" value={formatUsdc(BigInt(position.deposit))} />
          )}
          {BigInt(position.rootStake) > 0n && (
            <Stat label="Root stake" value={formatUsdc(BigInt(position.rootStake))} />
          )}
          {BigInt(position.childDebt) > 0n && (
            <Stat label="Child debt" value={formatUsdc(BigInt(position.childDebt))} warn />
          )}
          {BigInt(position.parentDebt) > 0n && (
            <Stat label="Parent debt" value={formatUsdc(BigInt(position.parentDebt))} warn />
          )}
        </div>

        {position.finalOutcome && (
          <p className="text-xs">
            Final outcome:{" "}
            <span
              className={
                position.finalOutcome === "Yes"
                  ? "text-green-600 dark:text-green-400 font-semibold"
                  : position.finalOutcome === "No"
                  ? "text-red-600 dark:text-red-400 font-semibold"
                  : "text-muted-foreground"
              }
            >
              {position.finalOutcome}
            </span>
          </p>
        )}

        {(BigInt(position.childDebt) > 0n || BigInt(position.parentDebt) > 0n) && (
          <Alert variant="warning">
            <AlertDescription>
              Position has encumbered debt. Transfers and sells may be blocked until debt is cleared.
            </AlertDescription>
          </Alert>
        )}

        {isRedeemable && onRedeem && (
          <Button
            size="xs"
            className="w-full"
            onClick={() => onRedeem(position)}
          >
            Redeem
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

function Stat({ label, value, warn }: { label: string; value: string; warn?: boolean }) {
  return (
    <div>
      <p className="text-muted-foreground">{label}</p>
      <p className={`font-medium ${warn ? "text-yellow-600 dark:text-yellow-400" : ""}`}>
        {value}
      </p>
    </div>
  );
}

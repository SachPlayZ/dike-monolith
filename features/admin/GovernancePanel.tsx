import type { AdminState, TimelockAction } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

interface GovernancePanelProps {
  state: AdminState;
  timelockActions: TimelockAction[];
}

export function GovernancePanel({ state, timelockActions }: GovernancePanelProps) {
  const queued = timelockActions.filter((a) => a.queued && !a.executed && !a.cancelled);
  const executed = timelockActions.filter((a) => a.executed);
  const cancelled = timelockActions.filter((a) => a.cancelled);

  return (
    <div className="space-y-6">
      {/* Protocol Config */}
      <Section title="Protocol Config">
        <Grid>
          <Kv label="Treasury" value={state.treasury} mono />
          <Kv
            label="Trading fee"
            value={state.feeConfig ? `${state.feeConfig.tradingFeeBps / 100}%` : "—"}
          />
          <Kv
            label="LP fee share"
            value={state.feeConfig ? `${state.feeConfig.lpFeeShareBps / 100}%` : "—"}
          />
          <Kv
            label="Treasury share"
            value={state.feeConfig ? `${state.feeConfig.treasuryFeeShareBps / 100}%` : "—"}
          />
          <Kv
            label="COD share"
            value={state.feeConfig ? `${state.feeConfig.codFeeShareBps / 100}%` : "—"}
          />
        </Grid>
      </Section>

      {/* Supported Collaterals */}
      <Section title="Supported Collaterals">
        {state.supportedCollaterals.length === 0 ? (
          <p className="text-xs text-muted-foreground">None</p>
        ) : (
          <ul className="space-y-1">
            {state.supportedCollaterals.map((c) => (
              <li key={c} className="font-mono text-xs">{c}</li>
            ))}
          </ul>
        )}
      </Section>

      {/* Module Addresses */}
      <Section title="Module Addresses">
        <Grid>
          {Object.entries(state.moduleAddresses).map(([k, v]) => (
            <Kv key={k} label={k} value={v} mono />
          ))}
        </Grid>
      </Section>

      {/* Approved Creators */}
      <Section title="Approved Creators">
        {state.approvedCreators.length === 0 ? (
          <p className="text-xs text-muted-foreground">None</p>
        ) : (
          <ul className="space-y-1">
            {state.approvedCreators.map((c) => (
              <li key={c} className="font-mono text-xs">{c}</li>
            ))}
          </ul>
        )}
      </Section>

      {/* Council Members */}
      <Section title="Council Members">
        {state.councilMembers.length === 0 ? (
          <p className="text-xs text-muted-foreground">None</p>
        ) : (
          <ul className="space-y-1">
            {state.councilMembers.map((m) => (
              <li key={m} className="font-mono text-xs">{m}</li>
            ))}
          </ul>
        )}
      </Section>

      {/* Timelock */}
      <Section title="Timelock Actions">
        {queued.length > 0 && (
          <div className="mb-4">
            <h4 className="text-xs font-semibold text-muted-foreground mb-2">Queued</h4>
            <ul className="space-y-2">
              {queued.map((a) => (
                <TimelockRow key={a.actionId} action={a} />
              ))}
            </ul>
          </div>
        )}
        {executed.length > 0 && (
          <div className="mb-4">
            <h4 className="text-xs font-semibold text-muted-foreground mb-2">Executed</h4>
            <ul className="space-y-2">
              {executed.map((a) => (
                <TimelockRow key={a.actionId} action={a} />
              ))}
            </ul>
          </div>
        )}
        {cancelled.length > 0 && (
          <div>
            <h4 className="text-xs font-semibold text-muted-foreground mb-2">Cancelled</h4>
            <ul className="space-y-2">
              {cancelled.map((a) => (
                <TimelockRow key={a.actionId} action={a} />
              ))}
            </ul>
          </div>
        )}
        {timelockActions.length === 0 && (
          <p className="text-xs text-muted-foreground">No timelock actions</p>
        )}
      </Section>
    </div>
  );
}

function TimelockRow({ action }: { action: TimelockAction }) {
  return (
    <li>
      <Card size="sm">
        <CardContent className="space-y-1 text-xs">
          <div className="flex justify-between">
            <span className="font-medium">{action.kind}</span>
            <span className="text-muted-foreground font-mono">
              {action.actionId.slice(0, 8)}…
            </span>
          </div>
          <p className="text-muted-foreground">
            ETA: {new Date(action.eta * 1000).toLocaleString()}
          </p>
          {action.data && (
            <p className="font-mono text-muted-foreground break-all">{action.data}</p>
          )}
        </CardContent>
      </Card>
    </li>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card size="sm">
      <CardHeader className="pb-0">
        <CardTitle className="text-sm font-semibold normal-case tracking-normal">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">{children}</CardContent>
    </Card>
  );
}

function Grid({ children }: { children: React.ReactNode }) {
  return <div className="grid grid-cols-2 gap-2">{children}</div>;
}

function Kv({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className={`text-xs font-medium break-all ${mono ? "font-mono" : ""}`}>{value}</p>
    </div>
  );
}

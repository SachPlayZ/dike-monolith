import { Suspense } from "react";
import { fetchGovernanceState, fetchTimelockActions } from "@/lib/api/admin";
import { GovernancePanel } from "@/features/admin/GovernancePanel";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { EmptyState } from "@/components/data-state/EmptyState";
import { ServiceUnavailableError } from "@/lib/api/client";

export const metadata = {
  title: "Admin — DIKE",
};

async function AdminContent() {
  try {
    const [state, timelockActions] = await Promise.all([
      fetchGovernanceState(),
      fetchTimelockActions(),
    ]);
    return <GovernancePanel state={state} timelockActions={timelockActions} />;
  } catch (e) {
    const msg =
      e instanceof ServiceUnavailableError
        ? "dike-services is not running. Start it to view governance state."
        : e instanceof Error
        ? e.message
        : "Failed to load admin state";
    return <EmptyState title="Unavailable" description={msg} />;
  }
}

export default function AdminPage() {
  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Admin / Governance</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Protocol configuration, timelock actions, and module state. Read-only for non-admin addresses.
        </p>
      </div>

      <Suspense fallback={<PageLoader />}>
          <AdminContent />
      </Suspense>
    </div>
  );
}

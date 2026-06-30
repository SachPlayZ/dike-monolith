import { Suspense } from "react";
import { fetchGovernanceState, fetchTimelockActions } from "@/lib/api/admin";
import { GovernancePanel } from "@/features/admin/GovernancePanel";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { EmptyState } from "@/components/data-state/EmptyState";
import { SectionGate } from "@/components/auth/SectionGate";
import { ServiceUnavailableError } from "@/lib/api/client";

export const dynamic = "force-dynamic";

export const metadata = {
  title: "Admin — DIKE",
};

async function loadAdminData() {
  try {
    const [state, timelockActions] = await Promise.all([
      fetchGovernanceState(),
      fetchTimelockActions(),
    ]);
    return { state, timelockActions, error: null as string | null };
  } catch (e) {
    const error =
      e instanceof ServiceUnavailableError
        ? "dike-services is not running. Start it to view governance state."
        : e instanceof Error
        ? e.message
        : "Failed to load admin state";
    return { state: null, timelockActions: null, error };
  }
}

async function AdminContent() {
  const { state, timelockActions, error } = await loadAdminData();
  if (error || !state || !timelockActions) {
    return <EmptyState title="Unavailable" description={error ?? "Failed to load admin state"} />;
  }

  return <GovernancePanel state={state} timelockActions={timelockActions} />;
}

export default function AdminPage() {
  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">Admin / Governance</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Protocol configuration, timelock actions, and module state. Read-only for non-admin addresses.
        </p>
      </div>

      <SectionGate
        permission="canAdmin"
        title="admin governance"
        description="Connect a governance-admin wallet to access protocol administration."
      >
        <Suspense fallback={<PageLoader />}>
          <AdminContent />
        </Suspense>
      </SectionGate>
    </div>
  );
}

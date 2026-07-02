import { Suspense } from "react";
import { fetchCouncilCases } from "@/lib/api/council";
import { CaseCard } from "@/features/council/CaseCard";
import { VoteForm } from "@/features/council/VoteForm";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { EmptyState } from "@/components/data-state/EmptyState";
import { SectionGate } from "@/components/auth/SectionGate";
import { ServiceUnavailableError } from "@/lib/api/client";
import type { CouncilCase } from "@/lib/types";

export const metadata = {
  title: "Council — DIKE",
};

async function CouncilCaseList() {
  // Council case reads are intentionally public protocol state. SectionGate is
  // a wallet-role UX guard, not a confidentiality boundary for this RSC fetch.
  let cases: CouncilCase[] = [];
  let error: string | null = null;

  try {
    cases = await fetchCouncilCases();
  } catch (e) {
    error =
      e instanceof ServiceUnavailableError
        ? "dike-services is not running."
        : e instanceof Error
        ? e.message
        : "Failed to load cases";
  }

  if (error) {
    return <EmptyState title="Unavailable" description={error} />;
  }

  if (cases.length === 0) {
    return (
      <EmptyState
        title="No active council cases"
        description="Disputed markets escalated to council will appear here."
      />
    );
  }

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      {cases.map((c) => (
        <div key={c.caseId} className="space-y-4">
          <CaseCard councilCase={c} />
          <VoteForm councilCase={c} />
        </div>
      ))}
    </div>
  );
}

export default function CouncilPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-heading text-3xl font-normal tracking-tight">Council of Dike</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Commit and reveal votes on disputed market outcomes. Your salt is stored locally — never loses it.
        </p>
      </div>

      <SectionGate
        permission="canCouncil"
        title="council cases"
        description="Connect a council-member wallet to access commit-reveal voting."
      >
        <Alert variant="warning">
          <AlertDescription>
            Commit-reveal voting: generate a random salt when committing, reveal it in the reveal phase. Losing your salt prevents you from revealing your vote.
          </AlertDescription>
        </Alert>

        <Suspense fallback={<PageLoader />}>
          <CouncilCaseList />
        </Suspense>
      </SectionGate>
    </div>
  );
}

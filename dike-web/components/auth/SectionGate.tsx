"use client";

import type { ReactNode } from "react";
import { useWallet } from "@/lib/contexts/wallet";
import { EmptyState } from "@/components/data-state/EmptyState";
import { PageLoader } from "@/components/data-state/LoadingSpinner";
import { Button } from "@/components/ui/button";

type RequiredPermission = "canResolve" | "canCouncil" | "canAdmin";

interface SectionGateProps {
  permission: RequiredPermission;
  title: string;
  description: string;
  children: ReactNode;
}

export function SectionGate({
  permission,
  title,
  description,
  children,
}: SectionGateProps) {
  const { isConnected, permissions, permissionsLoading, connect, isConnecting } = useWallet();

  if (!isConnected) {
    return (
      <EmptyState
        title="Connect your wallet"
        description={description}
        action={
          <Button size="sm" onClick={connect} disabled={isConnecting}>
            {isConnecting ? "Connecting…" : "Connect Wallet"}
          </Button>
        }
      />
    );
  }

  if (permissionsLoading) {
    return <PageLoader />;
  }

  if (!permissions?.[permission]) {
    return (
      <EmptyState
        title="Access restricted"
        description={`Connected wallet does not have permission to access ${title}.`}
      />
    );
  }

  return <>{children}</>;
}

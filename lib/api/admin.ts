import { apiGet } from "./client";
import { normalizeAdminState, normalizeTimelockAction } from "./normalizers";
import type { AdminState, TimelockAction } from "@/lib/types";

interface GovernanceResponse {
  config: Record<string, unknown> | null;
  lists: Record<string, unknown>[];
  modules: Record<string, unknown>[];
}

export async function fetchGovernanceState(): Promise<AdminState> {
  const response = await apiGet<GovernanceResponse>("/admin/governance");
  return normalizeAdminState(response);
}

export async function fetchTimelockActions(): Promise<TimelockAction[]> {
  const response = await apiGet<Record<string, unknown>[]>("/admin/timelock");
  return response.map((item) => normalizeTimelockAction(item));
}

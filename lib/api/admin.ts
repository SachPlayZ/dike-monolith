import { apiGet } from "./client";
import type { AdminState, TimelockAction } from "@/lib/types";

export async function fetchGovernanceState(): Promise<AdminState> {
  return apiGet<AdminState>("/admin/governance");
}

export async function fetchTimelockActions(): Promise<TimelockAction[]> {
  return apiGet<TimelockAction[]>("/admin/timelock");
}

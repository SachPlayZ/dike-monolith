import { apiGet } from "./client";
import { normalizeCouncilCase } from "./normalizers";
import type { CouncilCase } from "@/lib/types";

export async function fetchCouncilCases(): Promise<CouncilCase[]> {
  const res = await apiGet<Record<string, unknown>[]>("/council/cases");
  return res.map((item) => normalizeCouncilCase(item));
}

import { apiGet } from "./client";
import type { CouncilCase } from "@/lib/types";

export async function fetchCouncilCases(): Promise<CouncilCase[]> {
  return apiGet<CouncilCase[]>("/council/cases");
}

import { apiGet } from "./client";
import type { ResolutionRequest } from "@/lib/types";

export interface MarketResolution {
  marketId: string;
  request: ResolutionRequest | null;
}

export async function fetchMarketResolution(
  marketId: string
): Promise<MarketResolution> {
  return apiGet<MarketResolution>(`/markets/${marketId}/resolution`);
}

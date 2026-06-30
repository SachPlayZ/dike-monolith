import { apiGet } from "./client";
import { normalizeResolutionRequest } from "./normalizers";
import type { ResolutionRequest } from "@/lib/types";

export interface MarketResolution {
  marketId: string;
  request: ResolutionRequest | null;
}

export async function fetchMarketResolution(
  marketId: string
): Promise<MarketResolution> {
  const res = await apiGet<
    | MarketResolution
    | {
        market?: Record<string, unknown>;
        request?: Record<string, unknown> | null;
      }
  >(`/markets/${marketId}/resolution`);

  if ("request" in res || "market" in res) {
    const marketRow =
      "market" in res && res.market ? (res.market as Record<string, unknown>) : null;

    return {
      marketId: marketRow?.market_id != null ? String(marketRow.market_id) : marketId,
      request: normalizeResolutionRequest(("request" in res ? res.request ?? null : null) as
        | Record<string, unknown>
        | null),
    };
  }

  const normalized = res as MarketResolution;
  return {
    marketId: normalized.marketId,
    request: normalizeResolutionRequest(
      normalized.request as unknown as Record<string, unknown> | null,
    ),
  };
}

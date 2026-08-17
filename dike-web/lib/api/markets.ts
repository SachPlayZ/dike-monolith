import { apiGet } from "./client";
import { normalizeMarketData } from "./normalizers";
import type { MarketData } from "@/lib/types";

export async function fetchMarkets(): Promise<MarketData[]> {
  const res = await apiGet<{ items: Record<string, unknown>[]; nextCursor: string | null }>(
    "/markets",
  );
  return res.items.map((item) => normalizeMarketData(item));
}

export async function fetchMarket(id: string): Promise<MarketData> {
  const res = await apiGet<
    | Record<string, unknown>
    | {
        market?: Record<string, unknown>;
        liquidity?: Array<Record<string, unknown>>;
      }
  >(`/markets/${id}`);

  if ("market" in res && res.market) {
    const market = res.market as Record<string, unknown>;
    const pool =
      Array.isArray(res.liquidity) && res.liquidity.length > 0 ? res.liquidity[0] : {};
    // Pool data holds live reserves; spread last so pool wins over stale market fields
    return normalizeMarketData({ ...market, ...pool });
  }

  return normalizeMarketData(res);
}

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
    return normalizeMarketData(res.market as Record<string, unknown>);
  }

  return normalizeMarketData(res);
}

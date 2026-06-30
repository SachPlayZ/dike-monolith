import { apiGet } from "./client";
import type { MarketData } from "@/lib/types";

export async function fetchMarkets(): Promise<MarketData[]> {
  const res = await apiGet<{ items: MarketData[]; nextCursor: string | null }>("/markets");
  return res.items;
}

export async function fetchMarket(id: string): Promise<MarketData> {
  return apiGet<MarketData>(`/markets/${id}`);
}

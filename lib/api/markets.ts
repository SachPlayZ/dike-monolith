import { apiGet } from "./client";
import type { MarketData } from "@/lib/types";

export async function fetchMarkets(): Promise<MarketData[]> {
  return apiGet<MarketData[]>("/markets");
}

export async function fetchMarket(id: string): Promise<MarketData> {
  return apiGet<MarketData>(`/markets/${id}`);
}

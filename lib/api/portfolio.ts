import { apiGet } from "./client";
import { fetchMarkets } from "./markets";
import { normalizePortfolio } from "./normalizers";
import type { UserPosition } from "@/lib/types";

export async function fetchPortfolio(address: string): Promise<UserPosition[]> {
  const [portfolio, markets] = await Promise.all([
    apiGet<{
      positions: Record<string, unknown>[];
      lpPositions: Record<string, unknown>[];
      vaultState: Record<string, unknown>[];
    }>(`/users/${address}/portfolio`),
    fetchMarkets(),
  ]);

  return normalizePortfolio(portfolio, markets);
}

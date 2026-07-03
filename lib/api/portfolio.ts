import { apiGet } from "./client";
import { fetchMarkets } from "./markets";
import { normalizePortfolio } from "./normalizers";
import type { UserPosition } from "@/lib/types";

export interface RawPortfolio {
  positions: Record<string, unknown>[];
  lpPositions: Record<string, unknown>[];
  vaultState: Record<string, unknown>[];
}

// Unflattened indexer read — use this over fetchPortfolio when you need
// per-outcome fields (root_stake_no, parent_debt_no, ...) that
// normalizePortfolio collapses away.
export async function fetchRawPortfolio(address: string): Promise<RawPortfolio> {
  return apiGet<RawPortfolio>(`/users/${address}/portfolio`);
}

export async function fetchPortfolio(address: string): Promise<UserPosition[]> {
  const [portfolio, markets] = await Promise.all([
    fetchRawPortfolio(address),
    fetchMarkets(),
  ]);

  return normalizePortfolio(portfolio, markets);
}

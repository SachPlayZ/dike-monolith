import "server-only";

import { adminApiGet } from "./client";

export interface StatsTransaction {
  hash: string;
  ledger: string;
  topics: string[];
  eventCount: number;
  createdAt: string;
}

export interface ProtocolStats {
  connectedWallets: number;
  indexedWallets: number;
  transactionCount: number;
  transactions: StatsTransaction[];
}

export function fetchProtocolStats(): Promise<ProtocolStats> {
  return adminApiGet<ProtocolStats>("/admin/stats");
}

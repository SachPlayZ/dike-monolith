import { apiGet } from "./client";
import type { WalletPermissions } from "@/lib/types";

export async function fetchWalletPermissions(address: string): Promise<WalletPermissions> {
  return apiGet<WalletPermissions>(`/authz/${encodeURIComponent(address)}`);
}

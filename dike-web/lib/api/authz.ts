import { apiGet } from "./client";
import { normalizeWalletPermissions } from "./normalizers";
import type { WalletPermissions } from "@/lib/types";

export async function fetchWalletPermissions(address: string): Promise<WalletPermissions> {
  const response = await apiGet<Record<string, unknown>>(`/authz/${encodeURIComponent(address)}`);
  return normalizeWalletPermissions(response);
}

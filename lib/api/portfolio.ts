import { apiGet } from "./client";
import type { UserPosition } from "@/lib/types";

export async function fetchPortfolio(address: string): Promise<UserPosition[]> {
  return apiGet<UserPosition[]>(`/users/${address}/portfolio`);
}

import * as StellarSdk from "@stellar/stellar-sdk";

const SERVICES_URL = typeof window === "undefined"
  ? (process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000")
  : "/api/proxy";

export interface SponsorshipStatus {
  enabled: boolean;
  available: boolean;
  network: string;
  sponsorAddress: string | null;
  limits?: {
    baseFeeStroops: string;
    maxTotalFeeStroops: string;
    maxResourceFeeStroops: string;
  };
  reason: string | null;
}

export interface SponsoredTransactionResult {
  innerHash: string;
  outerHash: string;
  status: "SUCCESS";
  returnValueXdr?: string;
}

export class SponsorshipApiError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly retryable = false,
    readonly status = 500,
  ) {
    super(message);
    this.name = "SponsorshipApiError";
  }
}

async function readResponse<T>(response: Response): Promise<T> {
  const payload = await response.json().catch(() => null) as { error?: { code?: string; message?: string; retryable?: boolean } } | null;
  if (!response.ok) {
    const error = payload?.error;
    throw new SponsorshipApiError(
      error?.code ?? "SPONSORSHIP_UNAVAILABLE",
      error?.message ?? "Fee sponsorship is unavailable.",
      error?.retryable ?? response.status >= 500,
      response.status,
    );
  }
  return payload as T;
}

export async function getSponsorshipStatus(): Promise<SponsorshipStatus> {
  const response = await fetch(`${SERVICES_URL}/sponsorship/status`, { cache: "no-store" });
  return readResponse<SponsorshipStatus>(response);
}

export async function submitSponsoredTransaction(signedTransactionXdr: string): Promise<SponsoredTransactionResult> {
  const response = await fetch(`${SERVICES_URL}/sponsorship/transactions`, {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({ signedTransactionXdr }),
    cache: "no-store",
  });
  return readResponse<SponsoredTransactionResult>(response);
}

export function decodeSponsoredReturnValue(returnValueXdr?: string): StellarSdk.xdr.ScVal | undefined {
  if (!returnValueXdr) return undefined;
  return StellarSdk.xdr.ScVal.fromXDR(returnValueXdr, "base64");
}

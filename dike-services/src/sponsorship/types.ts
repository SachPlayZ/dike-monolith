import type * as StellarSdk from "@stellar/stellar-sdk";

export const SPONSORSHIP_CODES = [
  "SPONSORSHIP_DISABLED",
  "MALFORMED_XDR",
  "UNSUPPORTED_ENVELOPE",
  "UNSUPPORTED_SOURCE",
  "BAD_SIGNATURE",
  "EXPIRED_TRANSACTION",
  "DISALLOWED_CONTRACT",
  "DISALLOWED_METHOD",
  "FEE_LIMIT_EXCEEDED",
  "QUOTA_EXCEEDED",
  "BUDGET_EXCEEDED",
  "TRANSACTION_REPLAY",
  "RPC_REJECTED",
  "TRANSACTION_FAILED",
  "CONFIRMATION_TIMEOUT",
] as const;

export type SponsorshipCode = (typeof SPONSORSHIP_CODES)[number];

export class SponsorshipError extends Error {
  readonly name = "SponsorshipError";
  readonly retryable: boolean;

  constructor(
    readonly code: SponsorshipCode,
    message: string,
    options: { retryable?: boolean; cause?: unknown } = {},
  ) {
    super(message, { cause: options.cause });
    this.retryable = options.retryable ?? false;
  }
}

export interface SponsorshipRequest {
  signedTransactionXdr: string;
}

export interface SponsorshipResult {
  innerHash: string;
  outerHash: string;
  status: "SUCCESS";
  returnValueXdr?: string;
}

export interface ParsedSponsorshipTransaction {
  xdr: string;
  transaction: StellarSdk.Transaction;
  source: string;
  hash: string;
  contractId: string;
  method: string;
  innerFee: bigint;
  resourceFee: bigint;
  outerFee?: bigint;
}

export function publicSponsorshipError(error: unknown): {
  code: SponsorshipCode;
  message: string;
  retryable: boolean;
} {
  if (error instanceof SponsorshipError) {
    return {
      code: error.code,
      message: error.message,
      retryable: error.retryable,
    };
  }

  return {
    code: "RPC_REJECTED",
    message: "Sponsored transaction could not be submitted.",
    retryable: true,
  };
}

export function sponsorshipHttpStatus(code: SponsorshipCode) {
  if (code === "SPONSORSHIP_DISABLED") return 503;
  if (code === "QUOTA_EXCEEDED" || code === "BUDGET_EXCEEDED") return 429;
  if (code === "TRANSACTION_REPLAY") return 409;
  if (code === "RPC_REJECTED") return 502;
  if (code === "CONFIRMATION_TIMEOUT") return 504;
  if (code === "TRANSACTION_FAILED") return 422;
  return 400;
}

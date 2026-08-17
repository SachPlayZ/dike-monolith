import { z } from "zod";
import { SponsorshipError, type SponsorshipRequest } from "./types.js";

export const MAX_XDR_BYTES = 150_000;
export const MAX_XDR_BASE64_LENGTH = Math.ceil(MAX_XDR_BYTES / 3) * 4;

const requestSchema = z
  .object({
    signedTransactionXdr: z.string().min(1).max(MAX_XDR_BASE64_LENGTH),
  })
  .strict();

function isBase64(value: string) {
  return value.length % 4 === 0 && /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value);
}

export function parseSponsorshipRequest(body: unknown): SponsorshipRequest {
  const parsed = requestSchema.safeParse(body);
  if (!parsed.success || !isBase64(parsed.data.signedTransactionXdr)) {
    throw new SponsorshipError("MALFORMED_XDR", "signedTransactionXdr must be base64 XDR.");
  }

  const decoded = Buffer.from(parsed.data.signedTransactionXdr, "base64");
  if (decoded.length === 0 || decoded.length > MAX_XDR_BYTES) {
    throw new SponsorshipError("MALFORMED_XDR", "signedTransactionXdr exceeds the size limit.");
  }

  return parsed.data;
}

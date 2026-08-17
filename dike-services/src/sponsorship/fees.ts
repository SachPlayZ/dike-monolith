import { BASE_FEE } from "@stellar/stellar-sdk";
import { SponsorshipError, type ParsedSponsorshipTransaction } from "./types.js";

export interface FeePolicy {
  baseFeeStroops: string;
  maxTotalFeeStroops: string;
  maxResourceFeeStroops: string;
}

export interface FeeQuote {
  innerInclusionFee: bigint;
  outerFee: bigint;
  resourceFee: bigint;
}

export function quoteSponsoredFee(
  parsed: Pick<ParsedSponsorshipTransaction, "innerFee" | "resourceFee">,
  policy: FeePolicy,
): FeeQuote {
  const baseFee = BigInt(policy.baseFeeStroops);
  const maxTotalFee = BigInt(policy.maxTotalFeeStroops);
  const maxResourceFee = BigInt(policy.maxResourceFeeStroops);
  const innerInclusionFee = parsed.innerFee - parsed.resourceFee;

  if (baseFee < BigInt(BASE_FEE) || innerInclusionFee < BigInt(BASE_FEE) || innerInclusionFee > baseFee) {
    throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The inner inclusion fee is outside the sponsor policy.");
  }
  if (parsed.resourceFee < 0n || parsed.resourceFee > maxResourceFee) {
    throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The Soroban resource fee exceeds the sponsor policy.");
  }

  const outerFee = baseFee * 2n + parsed.resourceFee;
  if (outerFee < parsed.innerFee || outerFee > maxTotalFee) {
    throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The sponsored fee exceeds the sponsor policy.");
  }

  return { innerInclusionFee, outerFee, resourceFee: parsed.resourceFee };
}

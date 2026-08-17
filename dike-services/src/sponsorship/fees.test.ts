import { describe, expect, it } from "vitest";
import { quoteSponsoredFee, type FeePolicy } from "./fees.js";
import { SponsorshipError } from "./types.js";

const policy: FeePolicy = {
  baseFeeStroops: "2000000",
  maxTotalFeeStroops: "10000000",
  maxResourceFeeStroops: "8000000",
};

describe("quoteSponsoredFee", () => {
  it("includes the outer operation and resource fee", () => {
    expect(quoteSponsoredFee({ innerFee: 2_001_000n, resourceFee: 1_000n }, policy)).toEqual({
      innerInclusionFee: 2_000_000n,
      outerFee: 4_001_000n,
      resourceFee: 1_000n,
    });
  });

  it("rejects below minimum, over-cap, and resource-heavy fees", () => {
    for (const parsed of [
      { innerFee: 1_001n, resourceFee: 1_000n },
      { innerFee: 10_001_000n, resourceFee: 1_000n },
      { innerFee: 8_001_000n, resourceFee: 8_001_000n },
    ]) {
      expect(() => quoteSponsoredFee(parsed, policy)).toThrow(SponsorshipError);
    }
  });
});

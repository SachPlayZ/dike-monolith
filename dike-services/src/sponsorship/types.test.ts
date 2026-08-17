import { describe, expect, it } from "vitest";
import { SponsorshipError, publicSponsorshipError } from "./types.js";

describe("sponsorship errors", () => {
  it("serializes stable public error codes", () => {
    const error = new SponsorshipError("FEE_LIMIT_EXCEEDED", "fee is too high");
    expect(publicSponsorshipError(error)).toEqual({
      code: "FEE_LIMIT_EXCEEDED",
      message: "fee is too high",
      retryable: false,
    });
  });

  it("does not expose unknown internal error text", () => {
    expect(publicSponsorshipError(new Error("secret rpc payload"))).toEqual({
      code: "RPC_REJECTED",
      message: "Sponsored transaction could not be submitted.",
      retryable: true,
    });
  });
});

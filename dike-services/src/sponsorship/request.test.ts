import { describe, expect, it } from "vitest";
import { parseSponsorshipRequest } from "./request.js";
import { SponsorshipError } from "./types.js";

describe("parseSponsorshipRequest", () => {
  it("accepts a bounded base64 payload", () => {
    const signedTransactionXdr = Buffer.from("signed-xdr").toString("base64");
    expect(parseSponsorshipRequest({ signedTransactionXdr })).toEqual({ signedTransactionXdr });
  });

  it("rejects unknown fields, invalid base64, and empty bodies", () => {
    for (const body of [
      { signedTransactionXdr: "not base64!" },
      { signedTransactionXdr: "" },
      { signedTransactionXdr: "c2lnbmVkLXhkcg", extra: true },
      null,
    ]) {
      expect(() => parseSponsorshipRequest(body)).toThrow(SponsorshipError);
    }
  });
});

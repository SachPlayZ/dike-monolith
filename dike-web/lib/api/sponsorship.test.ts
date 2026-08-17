import { afterEach, describe, expect, it, vi } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import {
  decodeSponsoredReturnValue,
  getSponsorshipStatus,
  SponsorshipApiError,
  submitSponsoredTransaction,
} from "./sponsorship";

afterEach(() => vi.unstubAllGlobals());

describe("sponsorship api", () => {
  it("submits signed XDR and returns the outer result", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ innerHash: "i", outerHash: "o", status: "SUCCESS" }), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(submitSponsoredTransaction("signed")).resolves.toMatchObject({ outerHash: "o" });
    expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/sponsorship/transactions"), expect.objectContaining({ method: "POST" }));
  });

  it("maps service errors and decodes return values", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: { code: "QUOTA_EXCEEDED", message: "limit", retryable: false } }), { status: 429 })));
    await expect(getSponsorshipStatus()).rejects.toMatchObject({ code: "QUOTA_EXCEEDED", status: 429 });
    expect(() => decodeSponsoredReturnValue("bad")).toThrow();
    const val = StellarSdk.nativeToScVal(true);
    expect(decodeSponsoredReturnValue(val.toXDR("base64"))?.switch().name).toBe("scvBool");
    expect(new SponsorshipApiError("X", "x")).toBeInstanceOf(Error);
  });
});

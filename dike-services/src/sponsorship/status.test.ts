import { describe, expect, it } from "vitest";
import { FeeSponsorshipService } from "./service.js";

describe("fee sponsorship status", () => {
  it("reports disabled sponsorship without sponsor details", () => {
    const service = new FeeSponsorshipService({
      enabled: false,
      network: "testnet",
      networkPassphrase: "Test SDF Network ; September 2015",
      contracts: {} as never,
      feePolicy: { baseFeeStroops: "1", maxTotalFeeStroops: "1", maxResourceFeeStroops: "1" },
      signer: { publicKey: () => "", sign: () => {} },
      quota: {} as never,
      replay: {} as never,
      submitter: {} as never,
    });
    expect(service.status()).toMatchObject({ enabled: false, available: false, sponsorAddress: null, reason: "disabled" });
  });
});

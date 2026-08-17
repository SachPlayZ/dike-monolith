import { beforeEach, describe, expect, it, vi } from "vitest";
import { executeTransaction } from "./execute";
import { SponsorshipApiError } from "@/lib/api/sponsorship";

vi.mock("@/lib/api/sponsorship", () => ({
  getSponsorshipStatus: vi.fn(),
  submitSponsoredTransaction: vi.fn(),
  decodeSponsoredReturnValue: vi.fn(() => undefined),
  SponsorshipApiError: class SponsorshipApiError extends Error {},
}));
vi.mock("./transaction", () => ({
  submitAndPoll: vi.fn(),
  parseDikeError: (error: unknown) => error instanceof Error ? error.message : String(error),
}));

import { getSponsorshipStatus, submitSponsoredTransaction } from "@/lib/api/sponsorship";
import { submitAndPoll } from "./transaction";

describe("executeTransaction", () => {
  beforeEach(() => vi.clearAllMocks());

  it("uses the sponsor when available and preserves state transitions", async () => {
    vi.mocked(getSponsorshipStatus).mockResolvedValue({ enabled: true, available: true, network: "testnet", sponsorAddress: "G...", reason: null });
    vi.mocked(submitSponsoredTransaction).mockResolvedValue({ innerHash: "i", outerHash: "o", status: "SUCCESS" });
    const states: string[] = [];
    const result = await executeTransaction({
      build: vi.fn(async (mode) => { expect(mode).toBe("sponsored"); return "inner"; }),
      sign: vi.fn(async (_xdr, options) => { expect(options?.sponsored).toBe(true); return "signed"; }),
      method: "buy_yes",
      onState: (state) => states.push(state.status),
    });
    expect(result).toMatchObject({ hash: "o", sponsored: true });
    expect(states).toEqual(["building", "awaiting-signature", "sponsoring", "pending"]);
  });

  it("uses direct submit only when sponsorship is explicitly disabled", async () => {
    vi.mocked(getSponsorshipStatus).mockResolvedValue({ enabled: false, available: false, network: "testnet", sponsorAddress: null, reason: "disabled" });
    vi.mocked(submitAndPoll).mockResolvedValue({ hash: "direct" });
    const result = await executeTransaction({ build: async () => "inner", sign: async () => "signed" });
    expect(result).toMatchObject({ hash: "direct", sponsored: false });
  });

  it("does not silently direct-submit when sponsorship is degraded", async () => {
    vi.mocked(getSponsorshipStatus).mockResolvedValue({ enabled: true, available: false, network: "testnet", sponsorAddress: null, reason: "sponsor_not_configured" });
    await expect(executeTransaction({ build: async () => "inner", sign: async () => "signed" })).rejects.toBeInstanceOf(SponsorshipApiError);
    expect(submitAndPoll).not.toHaveBeenCalled();
  });
});

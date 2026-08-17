import { describe, expect, it } from "vitest";
import { MetricsStore } from "./metrics.js";

describe("sponsorship metrics", () => {
  it("tracks outcomes without exposing transaction payloads", () => {
    const metrics = new MetricsStore();
    metrics.noteSponsorshipRequested();
    metrics.noteSponsorshipAccepted(4_001_000n);
    metrics.noteSponsorshipCompleted("confirmed", 12);
    metrics.noteSponsorshipCompleted("rejected", 3, "FEE_LIMIT_EXCEEDED");
    expect(metrics.snapshot().sponsorship).toEqual({
      requested: 1,
      accepted: 1,
      confirmed: 1,
      failed: 0,
      timeout: 0,
      rejected: 1,
      declaredStroops: "4001000",
      latencyMs: 15,
      rejectionReasons: { FEE_LIMIT_EXCEEDED: 1 },
    });
  });
});

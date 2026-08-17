import { describe, expect, it } from "vitest";
import { RpcSponsorshipSubmitter, type SponsorshipRpc } from "./submitter.js";
import { SponsorshipError } from "./types.js";

function transactionStub() {
  return {} as Parameters<SponsorshipRpc["sendTransaction"]>[0];
}

describe("RpcSponsorshipSubmitter", () => {
  it("polls until success and returns the outer hash", async () => {
    let reads = 0;
    const rpc: SponsorshipRpc = {
      async sendTransaction() { return { status: "PENDING", hash: "outer-hash" }; },
      async getTransaction() {
        reads += 1;
        return reads === 1 ? { status: "NOT_FOUND" } : { status: "SUCCESS" };
      },
    };
    const submitter = new RpcSponsorshipSubmitter(rpc, { timeoutSeconds: 10, pollIntervalMs: 0 }, async () => {});
    await expect(submitter.submit(transactionStub())).resolves.toEqual({ outerHash: "outer-hash" });
  });

  it("maps rejection, failure, and timeout statuses", async () => {
    const rejected: SponsorshipRpc = {
      async sendTransaction() { return { status: "ERROR" }; },
      async getTransaction() { return { status: "NOT_FOUND" }; },
    };
    await expect(new RpcSponsorshipSubmitter(rejected).submit(transactionStub())).rejects.toThrow(SponsorshipError);

    const failed: SponsorshipRpc = {
      async sendTransaction() { return { status: "PENDING", hash: "hash" }; },
      async getTransaction() { return { status: "FAILED" }; },
    };
    await expect(new RpcSponsorshipSubmitter(failed).submit(transactionStub())).rejects.toThrow(/failed/);
  });
});

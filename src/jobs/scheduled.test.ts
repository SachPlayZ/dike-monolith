import { describe, expect, it, vi } from "vitest";
import { ScheduledJobs } from "./scheduled.js";

function makeJob(overrides?: {
  marketIds?: number[];
  reconcileVaultImpl?: ReturnType<typeof vi.fn>;
}) {
  const repository = {
    listKnownMarketIds: vi.fn().mockResolvedValue(overrides?.marketIds ?? [1, 2]),
    listKnownMarketPools: vi.fn().mockResolvedValue([]),
    listKnownPositionOwners: vi.fn().mockResolvedValue([]),
    listKnownLpOwners: vi.fn().mockResolvedValue([]),
    listKnownTimelockActionIds: vi.fn().mockResolvedValue([]),
  };

  const reconciliation = {
    reconcileGovernance: vi.fn().mockResolvedValue(undefined),
    reconcileMarket: vi.fn().mockResolvedValue(undefined),
    reconcileVault:
      overrides?.reconcileVaultImpl ??
      vi.fn().mockResolvedValue(undefined),
    reconcileUserPosition: vi.fn().mockResolvedValue(undefined),
    reconcileUserVaultState: vi.fn().mockResolvedValue(undefined),
    reconcileLpPosition: vi.fn().mockResolvedValue(undefined),
    reconcileTimelockAction: vi.fn().mockResolvedValue(undefined),
  };

  const contracts = {
    getLatestLedger: vi.fn().mockResolvedValue({ sequence: 100 }),
  };

  const logger = {
    error: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
  };

  const job = new ScheduledJobs(
    {} as never,
    "mainnet",
    repository as never,
    reconciliation as never,
    contracts as never,
    logger as never,
  );

  return { job, repository, reconciliation, contracts, logger };
}

describe("ScheduledJobs.tick", () => {
  it("still reconciles later markets when an earlier market's vault reconciliation throws", async () => {
    const reconcileVault = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Cannot read properties of undefined (reading 'name')"))
      .mockResolvedValue(undefined);

    const { job, reconciliation, logger } = makeJob({
      marketIds: [1, 2],
      reconcileVaultImpl: reconcileVault,
    });

    await job.tick();

    expect(reconciliation.reconcileVault).toHaveBeenCalledTimes(2);
    expect(reconciliation.reconcileVault).toHaveBeenNthCalledWith(1, 1, 100);
    expect(reconciliation.reconcileVault).toHaveBeenNthCalledWith(2, 2, 100);
    expect(reconciliation.reconcileMarket).toHaveBeenCalledTimes(2);
    expect(logger.error).toHaveBeenCalledWith(
      expect.objectContaining({ scope: "vault", marketId: 1 }),
      "Reconciliation step failed; continuing with remaining items",
    );
  });

  it("does not throw out of tick() when every step fails", async () => {
    const { job, reconciliation, logger } = makeJob({
      reconcileVaultImpl: vi.fn().mockRejectedValue(new Error("boom")),
    });

    await expect(job.tick()).resolves.toBeUndefined();
    expect(logger.error).toHaveBeenCalled();
  });
});

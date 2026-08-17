import { describe, expect, it } from "vitest";
import { RedisSponsorshipQuota, sponsorshipReserveScript, type RedisEvalClient } from "./quota.js";
import { SponsorshipError } from "./types.js";

class FakeRedis implements RedisEvalClient {
  readonly calls: string[][] = [];
  constructor(private readonly result: unknown) {}
  async eval(_script: string, _numberOfKeys: number, ...args: string[]) {
    this.calls.push(args);
    return this.result;
  }
}

describe("RedisSponsorshipQuota", () => {
  it("uses an atomic Lua script and returns a daily reservation", async () => {
    const redis = new FakeRedis([1, "ok"]);
    const quota = new RedisSponsorshipQuota(redis, {
      maxPerMinute: 10,
      maxPerIpMinute: 30,
      dailyBudgetStroops: "1000000",
      ttlSeconds: 120,
    });

    const reservation = await quota.reserve("Gsource", "127.0.0.1", 123n, 0);
    expect(reservation.amount).toBe(123n);
    expect(redis.calls[0]?.[0]).toContain("dike:sponsor:source");
    expect(sponsorshipReserveScript).toContain("INCRBY");
  });

  it("maps quota and budget results to stable errors", async () => {
    const quota = (result: unknown) => new RedisSponsorshipQuota(
      new FakeRedis(result),
      { maxPerMinute: 1, maxPerIpMinute: 1, dailyBudgetStroops: "1", ttlSeconds: 1 },
    );
    await expect(quota([0, "source"]).reserve("G", "ip", 1n)).rejects.toThrow(SponsorshipError);
    await expect(quota([0, "budget"]).reserve("G", "ip", 1n)).rejects.toThrow(/limit/);
  });
});

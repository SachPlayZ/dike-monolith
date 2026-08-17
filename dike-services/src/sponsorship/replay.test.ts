import { describe, expect, it } from "vitest";
import { RedisSponsorshipReplay, type RedisReplayClient } from "./replay.js";
import { SponsorshipError, type SponsorshipResult } from "./types.js";

class MemoryRedis implements RedisReplayClient {
  private readonly values = new Map<string, string>();
  async get(key: string) { return this.values.get(key) ?? null; }
  async set(key: string, value: string, ...args: Array<string | number>) {
    if (args.includes("NX") && this.values.has(key)) return null;
    this.values.set(key, value);
    return "OK";
  }
  async del(key: string) { this.values.delete(key); }
}

const result: SponsorshipResult = { innerHash: "inner", outerHash: "outer", status: "SUCCESS" };

describe("RedisSponsorshipReplay", () => {
  it("allows one owner and caches terminal results", async () => {
    const replay = new RedisSponsorshipReplay(new MemoryRedis(), 60, 10);
    const token = await replay.acquire("inner");
    await expect(replay.acquire("inner")).rejects.toThrow(SponsorshipError);
    await replay.complete("inner", token, { status: "success", result });
    expect(await replay.terminal("inner")).toEqual({ status: "success", result });
  });

  it("releases a pre-submit lock", async () => {
    const replay = new RedisSponsorshipReplay(new MemoryRedis(), 60, 10);
    const token = await replay.acquire("inner");
    await replay.release("inner", token);
    await expect(replay.acquire("inner")).resolves.toBeTypeOf("string");
  });
});

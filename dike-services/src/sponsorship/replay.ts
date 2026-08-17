import { randomUUID } from "node:crypto";
import { SponsorshipError, type SponsorshipCode, type SponsorshipResult } from "./types.js";

export interface RedisReplayClient {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ...args: Array<string | number>): Promise<unknown>;
  del(key: string): Promise<unknown>;
}

export interface ReplayFailure {
  status: "failure";
  code: SponsorshipCode;
}

export interface ReplaySuccess {
  status: "success";
  result: SponsorshipResult;
}

export type ReplayRecord = ReplayFailure | ReplaySuccess;

export class RedisSponsorshipReplay {
  constructor(
    private readonly redis: RedisReplayClient,
    private readonly ttlSeconds: number,
    private readonly lockTtlSeconds: number,
  ) {}

  private resultKey(innerHash: string) {
    return `dike:sponsor:result:${innerHash}`;
  }

  private lockKey(innerHash: string) {
    return `dike:sponsor:lock:${innerHash}`;
  }

  async terminal(innerHash: string): Promise<ReplayRecord | null> {
    const value = await this.redis.get(this.resultKey(innerHash));
    return value ? JSON.parse(value) as ReplayRecord : null;
  }

  async acquire(innerHash: string): Promise<string> {
    const token = randomUUID();
    const acquired = await this.redis.set(
      this.lockKey(innerHash),
      token,
      "NX",
      "EX",
      this.lockTtlSeconds,
    );
    if (acquired !== "OK") {
      throw new SponsorshipError("TRANSACTION_REPLAY", "This transaction is already being sponsored.", { retryable: true });
    }
    return token;
  }

  async complete(innerHash: string, token: string, record: ReplayRecord) {
    const lockKey = this.lockKey(innerHash);
    const current = await this.redis.get(lockKey);
    if (current !== token) {
      throw new SponsorshipError("TRANSACTION_REPLAY", "The sponsorship lock is no longer owned.");
    }
    await this.redis.set(this.resultKey(innerHash), JSON.stringify(record), "EX", this.ttlSeconds);
    await this.redis.del(lockKey);
  }

  async release(innerHash: string, token: string) {
    const lockKey = this.lockKey(innerHash);
    if (await this.redis.get(lockKey) === token) await this.redis.del(lockKey);
  }
}

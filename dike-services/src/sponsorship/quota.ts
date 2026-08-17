import { SponsorshipError } from "./types.js";

export interface RedisEvalClient {
  eval(script: string, numberOfKeys: number, ...args: string[]): Promise<unknown>;
}

export interface QuotaPolicy {
  maxPerMinute: number;
  maxPerIpMinute: number;
  dailyBudgetStroops: string;
  ttlSeconds: number;
}

export interface BudgetReservation {
  dailyKey: string;
  amount: bigint;
}

const RESERVE_SCRIPT = `
local source_count = redis.call('INCRBY', KEYS[1], 1)
if source_count == 1 then redis.call('EXPIRE', KEYS[1], ARGV[5]) end
if source_count > tonumber(ARGV[2]) then redis.call('DECRBY', KEYS[1], 1); return {0, 'source'} end
local ip_count = redis.call('INCRBY', KEYS[2], 1)
if ip_count == 1 then redis.call('EXPIRE', KEYS[2], ARGV[5]) end
if ip_count > tonumber(ARGV[3]) then redis.call('DECRBY', KEYS[1], 1); redis.call('DECRBY', KEYS[2], 1); return {0, 'ip'} end
local daily_count = redis.call('INCRBY', KEYS[3], ARGV[1])
if daily_count == tonumber(ARGV[1]) then redis.call('EXPIRE', KEYS[3], ARGV[6]) end
if daily_count > tonumber(ARGV[4]) then redis.call('DECRBY', KEYS[1], 1); redis.call('DECRBY', KEYS[2], 1); redis.call('DECRBY', KEYS[3], ARGV[1]); return {0, 'budget'} end
return {1, 'ok'}
`;

const RELEASE_SCRIPT = `
local value = redis.call('DECRBY', KEYS[1], ARGV[1])
if value < 0 then redis.call('SET', KEYS[1], '0', 'EX', ARGV[2]) end
return value
`;

function keyPart(value: string) {
  return encodeURIComponent(value.trim().toUpperCase());
}

export class RedisSponsorshipQuota {
  constructor(
    private readonly redis: RedisEvalClient,
    private readonly policy: QuotaPolicy,
  ) {}

  async reserve(source: string, ip: string, amount: bigint, nowMs = Date.now()): Promise<BudgetReservation> {
    if (amount <= 0n) throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The declared fee must be positive.");
    const minute = Math.floor(nowMs / 60_000);
    const day = new Date(nowMs).toISOString().slice(0, 10);
    const sourceKey = `dike:sponsor:source:${keyPart(source)}:${minute}`;
    const ipKey = `dike:sponsor:ip:${keyPart(ip)}:${minute}`;
    const dailyKey = `dike:sponsor:budget:${day}`;
    const result = await this.redis.eval(
      RESERVE_SCRIPT,
      3,
      sourceKey,
      ipKey,
      dailyKey,
      amount.toString(),
      String(this.policy.maxPerMinute),
      String(this.policy.maxPerIpMinute),
      this.policy.dailyBudgetStroops,
      String(this.policy.ttlSeconds),
      String(2 * 24 * 60 * 60),
    );
    const tuple = Array.isArray(result) ? result : [];
    if (String(tuple[0]) !== "1") {
      const reason = String(tuple[1]);
      throw new SponsorshipError(reason === "budget" ? "BUDGET_EXCEEDED" : "QUOTA_EXCEEDED", "Sponsorship limit reached.");
    }
    return { dailyKey, amount };
  }

  async releaseBudget(reservation: BudgetReservation) {
    await this.redis.eval(
      RELEASE_SCRIPT,
      1,
      reservation.dailyKey,
      reservation.amount.toString(),
      String(2 * 24 * 60 * 60),
    );
  }
}

export const sponsorshipReserveScript = RESERVE_SCRIPT;

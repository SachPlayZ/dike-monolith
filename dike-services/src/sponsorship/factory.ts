import * as StellarSdk from "@stellar/stellar-sdk";
import type { Env } from "../config/env.js";
import type { LoadedManifest } from "../config/manifest.js";
import { RedisSponsorshipQuota, type RedisEvalClient } from "./quota.js";
import { RedisSponsorshipReplay, type RedisReplayClient } from "./replay.js";
import { FeeSponsorshipService } from "./service.js";
import { createSeedSponsorSigner, type SponsorSigner } from "./signer.js";
import { RpcSponsorshipSubmitter } from "./submitter.js";
import type { MetricsStore } from "../observability/metrics.js";
import type { Logger } from "../observability/logger.js";

type RedisClient = RedisEvalClient & RedisReplayClient;

function disabledSigner(): SponsorSigner {
  return {
    publicKey: () => "",
    sign: () => {},
  };
}

export function createFeeSponsorshipService(
  env: Env,
  manifest: LoadedManifest,
  redis: RedisClient,
  rpc: StellarSdk.rpc.Server,
  metrics?: MetricsStore,
  logger?: Logger,
) {
  const signer = env.FEE_SPONSOR_SEED
    ? createSeedSponsorSigner(env.FEE_SPONSOR_SEED)
    : disabledSigner();
  return new FeeSponsorshipService({
    enabled: env.FEE_SPONSOR_ENABLED,
    network: env.STELLAR_NETWORK,
    networkPassphrase: env.STELLAR_NETWORK_PASSPHRASE,
    contracts: manifest.data.contracts,
    feePolicy: {
      baseFeeStroops: env.FEE_SPONSOR_BASE_FEE_STROOPS,
      maxTotalFeeStroops: env.FEE_SPONSOR_MAX_TOTAL_FEE_STROOPS,
      maxResourceFeeStroops: env.FEE_SPONSOR_MAX_RESOURCE_FEE_STROOPS,
    },
    signer,
    quota: new RedisSponsorshipQuota(redis, {
      maxPerMinute: env.FEE_SPONSOR_MAX_PER_MINUTE,
      maxPerIpMinute: env.FEE_SPONSOR_MAX_PER_IP_MINUTE,
      dailyBudgetStroops: env.FEE_SPONSOR_DAILY_BUDGET_STROOPS,
      ttlSeconds: env.FEE_SPONSOR_REPLAY_TTL_SECONDS,
    }),
    replay: new RedisSponsorshipReplay(
      redis,
      env.FEE_SPONSOR_REPLAY_TTL_SECONDS,
      env.FEE_SPONSOR_LOCK_TTL_SECONDS,
    ),
    submitter: new RpcSponsorshipSubmitter(rpc, {
      timeoutSeconds: env.FEE_SPONSOR_CONFIRMATION_TIMEOUT_SECONDS,
    }),
    ...(metrics ? { metrics } : {}),
    ...(logger ? { logger } : {}),
  });
}

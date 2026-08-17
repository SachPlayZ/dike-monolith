import "dotenv/config";
import { z } from "zod";
import { NETWORK_PASSPHRASES, type StellarNetworkName } from "./networks.js";

const MAX_STROOPS = 9_223_372_036_854_775_807n;

const stroops = (defaultValue: string) =>
  z
    .string()
    .regex(/^\d+$/, "must be a non-negative integer number of stroops")
    .refine((value) => BigInt(value) <= MAX_STROOPS, "stroop value is too large")
    .default(defaultValue);

const envBoolean = z.preprocess(
  (value) => {
    if (value === undefined) return false;
    if (value === true || value === "true" || value === "1") return true;
    if (value === false || value === "false" || value === "0") return false;
    return value;
  },
  z.boolean(),
);

const envSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]).default("development"),
  PORT: z.coerce.number().int().positive().default(4000),
  STELLAR_NETWORK: z.enum(["mainnet", "testnet", "local"]).default("testnet"),
  STELLAR_RPC_URL: z.url(),
  STELLAR_HORIZON_URL: z.url(),
  STELLAR_NETWORK_PASSPHRASE: z.string().min(1),
  DIKE_CONTRACTS_ROOT: z.string().min(1).default(process.cwd()),
  DIKE_MANIFEST_PATH: z.string().min(1).default("./deployments/testnet.json"),
  DATABASE_URL: z.string().min(1),
  REDIS_URL: z.string().min(1),
  ADMIN_API_KEY: z.string().min(1).optional(),
  INDEXER_START_LEDGER: z
    .string()
    .optional()
    .transform((value) => (value ? Number(value) : undefined))
    .pipe(z.number().int().positive().optional()),
  INDEXER_POLL_INTERVAL_MS: z.coerce.number().int().positive().default(5_000),
  INDEXER_LEDGER_WINDOW: z.coerce.number().int().positive().default(250),
  RECONCILIATION_INTERVAL_MS: z.coerce.number().int().positive().default(60_000),
  INDEXER_LAG_ALERT_THRESHOLD: z.coerce.number().int().positive().default(50),
  REORG_SAFETY_MARGIN_LEDGERS: z.coerce.number().int().min(0).default(10),
  FEE_SPONSOR_ENABLED: envBoolean.default(false),
  FEE_SPONSOR_SEED: z.string().min(1).optional(),
  FEE_SPONSOR_BASE_FEE_STROOPS: stroops("2000000"),
  FEE_SPONSOR_MAX_TOTAL_FEE_STROOPS: stroops("10000000"),
  FEE_SPONSOR_MAX_RESOURCE_FEE_STROOPS: stroops("8000000"),
  FEE_SPONSOR_MAX_PER_MINUTE: z.coerce.number().int().positive().default(10),
  FEE_SPONSOR_MAX_PER_IP_MINUTE: z.coerce.number().int().positive().default(30),
  FEE_SPONSOR_DAILY_BUDGET_STROOPS: stroops("100000000"),
  FEE_SPONSOR_REPLAY_TTL_SECONDS: z.coerce.number().int().positive().default(900),
  FEE_SPONSOR_LOCK_TTL_SECONDS: z.coerce.number().int().positive().default(30),
  FEE_SPONSOR_CONFIRMATION_TIMEOUT_SECONDS: z.coerce.number().int().positive().default(120),
});

export type Env = z.infer<typeof envSchema> & {
  STELLAR_NETWORK: StellarNetworkName;
};

export function loadEnv(): Env {
  const env = envSchema.parse(process.env);
  const expectedPassphrase = NETWORK_PASSPHRASES[env.STELLAR_NETWORK];

  if (env.STELLAR_NETWORK_PASSPHRASE !== expectedPassphrase) {
    throw new Error(
      `Passphrase mismatch for ${env.STELLAR_NETWORK}. Expected "${expectedPassphrase}".`,
    );
  }

  if (env.FEE_SPONSOR_ENABLED && !env.FEE_SPONSOR_SEED) {
    throw new Error("FEE_SPONSOR_SEED is required when fee sponsorship is enabled.");
  }

  return env as Env;
}

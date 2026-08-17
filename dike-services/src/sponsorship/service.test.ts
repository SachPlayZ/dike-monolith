import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { FeeSponsorshipService } from "./service.js";
import { RedisSponsorshipReplay, type RedisReplayClient } from "./replay.js";
import { createSeedSponsorSigner } from "./signer.js";

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

function makeRequest() {
  const source = StellarSdk.Keypair.random();
  const sponsor = StellarSdk.Keypair.random();
  const amm = StellarSdk.StrKey.encodeContract(Buffer.from("amm-contract".padEnd(32, "0")));
  const data = new StellarSdk.SorobanDataBuilder().setResources(10_000, 100, 100).setResourceFee("1000").build();
  const transaction = new StellarSdk.TransactionBuilder(
    new StellarSdk.Account(source.publicKey(), "1"),
    { fee: "2000000", networkPassphrase: StellarSdk.Networks.TESTNET, sorobanData: data },
  )
    .addOperation(new StellarSdk.Contract(amm).call("buy_yes"))
    .setTimeout(300)
    .build();
  transaction.sign(source);
  return { source, sponsor, amm, signedTransactionXdr: transaction.toXDR() };
}

function makeService(enabled = true) {
  const request = makeRequest();
  const redis = new MemoryRedis();
  const replay = new RedisSponsorshipReplay(redis, 60, 10);
  const signer = createSeedSponsorSigner(request.sponsor.secret());
  const quota = {
    async reserve() { return { dailyKey: "budget", amount: 4_001_000n }; },
    async releaseBudget() {},
  } as never;
  const submitter = {
    async submit() { return { outerHash: "outer-hash" }; },
  } as never;
  const contracts = Object.fromEntries([
    ["mock_usdc", ""], ["dike_timelock", ""], ["dike_governance", ""],
    ["market_registry", ""], ["conditional_tokens", ""], ["collateral_vault", ""],
    ["amm", request.amm], ["fee_manager", ""], ["cod_oracle", ""],
    ["council_of_dike", ""], ["market_factory", ""],
  ]) as never;
  return {
    request,
    service: new FeeSponsorshipService({
      enabled,
      network: "testnet",
      networkPassphrase: StellarSdk.Networks.TESTNET,
      contracts,
      feePolicy: { baseFeeStroops: "2000000", maxTotalFeeStroops: "10000000", maxResourceFeeStroops: "8000000" },
      signer,
      quota,
      replay,
      submitter,
    }),
  };
}

describe("FeeSponsorshipService", () => {
  it("sponsors a validated request and returns the outer hash", async () => {
    const { service, request } = makeService();
    await expect(service.sponsor({ signedTransactionXdr: request.signedTransactionXdr }, "127.0.0.1"))
      .resolves.toMatchObject({ outerHash: "outer-hash", status: "SUCCESS" });
  });

  it("rejects disabled sponsorship before decoding input", async () => {
    const { service } = makeService(false);
    await expect(service.sponsor({}, "127.0.0.1")).rejects.toThrow(/not enabled/);
  });
});

import { afterEach, describe, expect, it } from "vitest";
import { loadEnv } from "./env.js";

const previousEnv = { ...process.env };

afterEach(() => {
  process.env = { ...previousEnv };
});

describe("loadEnv", () => {
  it("accepts a matching network and passphrase", () => {
    process.env = {
      ...previousEnv,
      NODE_ENV: "test",
      PORT: "4000",
      STELLAR_NETWORK: "testnet",
      STELLAR_RPC_URL: "https://soroban-testnet.stellar.org",
      STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
      STELLAR_NETWORK_PASSPHRASE: "Test SDF Network ; September 2015",
      DIKE_CONTRACTS_ROOT: "/tmp/contracts",
      DIKE_MANIFEST_PATH: "/tmp/contracts/deployments/testnet.json",
      DATABASE_URL: "postgres://postgres:postgres@localhost:5432/dike_services",
      REDIS_URL: "redis://localhost:6379",
    };

    expect(loadEnv().STELLAR_NETWORK).toBe("testnet");
  });

  it("fails fast on network passphrase mismatch", () => {
    process.env = {
      ...previousEnv,
      NODE_ENV: "test",
      PORT: "4000",
      STELLAR_NETWORK: "testnet",
      STELLAR_RPC_URL: "https://soroban-testnet.stellar.org",
      STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
      STELLAR_NETWORK_PASSPHRASE: "Standalone Network ; February 2017",
      DIKE_CONTRACTS_ROOT: "/tmp/contracts",
      DIKE_MANIFEST_PATH: "/tmp/contracts/deployments/testnet.json",
      DATABASE_URL: "postgres://postgres:postgres@localhost:5432/dike_services",
      REDIS_URL: "redis://localhost:6379",
    };

    expect(() => loadEnv()).toThrow(/Passphrase mismatch/);
  });

  it("rejects zero as an indexer start ledger", () => {
    process.env = {
      ...previousEnv,
      NODE_ENV: "test",
      PORT: "4000",
      STELLAR_NETWORK: "testnet",
      STELLAR_RPC_URL: "https://soroban-testnet.stellar.org",
      STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
      STELLAR_NETWORK_PASSPHRASE: "Test SDF Network ; September 2015",
      DIKE_CONTRACTS_ROOT: "/tmp/contracts",
      DIKE_MANIFEST_PATH: "/tmp/contracts/deployments/testnet.json",
      DATABASE_URL: "postgres://postgres:postgres@localhost:5432/dike_services",
      REDIS_URL: "redis://localhost:6379",
      INDEXER_START_LEDGER: "0",
    };

    expect(() => loadEnv()).toThrow();
  });

  it("requires a sponsor seed when sponsorship is enabled", () => {
    process.env = {
      ...previousEnv,
      NODE_ENV: "test",
      PORT: "4000",
      STELLAR_NETWORK: "testnet",
      STELLAR_RPC_URL: "https://soroban-testnet.stellar.org",
      STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
      STELLAR_NETWORK_PASSPHRASE: "Test SDF Network ; September 2015",
      DIKE_CONTRACTS_ROOT: "/tmp/contracts",
      DIKE_MANIFEST_PATH: "/tmp/contracts/deployments/testnet.json",
      DATABASE_URL: "postgres://postgres:postgres@localhost:5432/dike_services",
      REDIS_URL: "redis://localhost:6379",
      FEE_SPONSOR_ENABLED: "true",
    };

    expect(() => loadEnv()).toThrow(/FEE_SPONSOR_SEED/);
  });

  it("keeps sponsorship stroop limits as strings", () => {
    process.env = {
      ...previousEnv,
      NODE_ENV: "test",
      PORT: "4000",
      STELLAR_NETWORK: "testnet",
      STELLAR_RPC_URL: "https://soroban-testnet.stellar.org",
      STELLAR_HORIZON_URL: "https://horizon-testnet.stellar.org",
      STELLAR_NETWORK_PASSPHRASE: "Test SDF Network ; September 2015",
      DIKE_CONTRACTS_ROOT: "/tmp/contracts",
      DIKE_MANIFEST_PATH: "/tmp/contracts/deployments/testnet.json",
      DATABASE_URL: "postgres://postgres:postgres@localhost:5432/dike_services",
      REDIS_URL: "redis://localhost:6379",
      FEE_SPONSOR_ENABLED: "true",
      FEE_SPONSOR_SEED: "SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
      FEE_SPONSOR_BASE_FEE_STROOPS: "2000000000000000",
    };

    expect(loadEnv().FEE_SPONSOR_BASE_FEE_STROOPS).toBe("2000000000000000");
  });
});

import * as StellarSdk from "@stellar/stellar-sdk";

export type Network = "testnet" | "mainnet";

export const NETWORK = (
  process.env.NEXT_PUBLIC_STELLAR_NETWORK ?? "testnet"
) as Network;

export const networkConfig = {
  network: NETWORK,
  rpcUrl:
    process.env.NEXT_PUBLIC_STELLAR_RPC_URL ??
    "https://soroban-testnet.stellar.org",
  horizonUrl:
    process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL ??
    "https://horizon-testnet.stellar.org",
  networkPassphrase:
    process.env.NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE ??
    StellarSdk.Networks.TESTNET,
  manifestNetwork:
    process.env.NEXT_PUBLIC_DIKE_MANIFEST_NETWORK ?? "testnet",
  servicesUrl:
    process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000",
} as const;

export function assertTestnet() {
  if (NETWORK !== "testnet") {
    throw new Error(
      `Expected testnet but wallet is on ${NETWORK}. Switch your wallet to Stellar Testnet.`
    );
  }
}

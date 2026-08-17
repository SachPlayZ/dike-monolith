import * as StellarSdk from "@stellar/stellar-sdk";

export type Network = "testnet" | "mainnet";

const rawNetwork = process.env.NEXT_PUBLIC_STELLAR_NETWORK ?? "testnet";
export const NETWORK: Network = rawNetwork === "mainnet" ? "mainnet" : "testnet";
const expectedPassphrase =
  NETWORK === "mainnet" ? StellarSdk.Networks.PUBLIC : StellarSdk.Networks.TESTNET;
const manifestNetwork = process.env.NEXT_PUBLIC_DIKE_MANIFEST_NETWORK ?? NETWORK;

function validHttpUrl(value: string) {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export const networkConfig = {
  network: NETWORK,
  explorerNetwork: NETWORK === "mainnet" ? "public" : "testnet",
  label: NETWORK === "mainnet" ? "Stellar Mainnet" : "Stellar Testnet",
  rpcUrl:
    process.env.NEXT_PUBLIC_STELLAR_RPC_URL ??
    (NETWORK === "mainnet"
      ? "https://soroban-rpc.mainnet.stellar.gateway.fm"
      : "https://soroban-testnet.stellar.org"),
  horizonUrl:
    process.env.NEXT_PUBLIC_STELLAR_HORIZON_URL ??
    (NETWORK === "mainnet"
      ? "https://horizon.stellar.org"
      : "https://horizon-testnet.stellar.org"),
  networkPassphrase:
    process.env.NEXT_PUBLIC_STELLAR_NETWORK_PASSPHRASE ?? expectedPassphrase,
  manifestNetwork,
  servicesUrl:
    process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000",
} as const;

export const configurationErrors = [
  ...(rawNetwork === "testnet" || rawNetwork === "mainnet"
    ? []
    : [`NEXT_PUBLIC_STELLAR_NETWORK must be "testnet" or "mainnet" (received "${rawNetwork}").`]),
  ...(manifestNetwork === NETWORK
    ? []
    : [`Manifest network "${manifestNetwork}" does not match Stellar network "${NETWORK}".`]),
  ...(networkConfig.networkPassphrase === expectedPassphrase
    ? []
    : [`Network passphrase does not match ${networkConfig.label}.`]),
  ...(validHttpUrl(networkConfig.rpcUrl) ? [] : ["Stellar RPC URL must be an HTTP(S) URL."]),
  ...(validHttpUrl(networkConfig.horizonUrl) ? [] : ["Horizon URL must be an HTTP(S) URL."]),
  ...(validHttpUrl(networkConfig.servicesUrl) ? [] : ["Dike services URL must be an HTTP(S) URL."]),
];

export function assertValidConfiguration() {
  if (configurationErrors.length > 0) {
    throw new Error(`Invalid network configuration: ${configurationErrors.join(" ")}`);
  }
}

export class NetworkMismatchError extends Error {
  constructor(source: string, observedNetwork: string) {
    super(`${source} is serving "${observedNetwork}" data, but this app is configured for "${NETWORK}".`);
    this.name = "NetworkMismatchError";
  }
}

export function assertObservedNetwork(value: unknown, source: string) {
  if (typeof value === "string" && value.length > 0 && value !== NETWORK) {
    throw new NetworkMismatchError(source, value);
  }
}

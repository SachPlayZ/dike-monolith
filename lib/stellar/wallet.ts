"use client";

import { networkConfig } from "./config";

// Thin module-level init guard for StellarWalletsKit (static class).
// Call initWalletKit() once before using any kit methods.

let _initialized = false;

export async function initWalletKit(): Promise<void> {
  if (_initialized || typeof window === "undefined") return;

  const { StellarWalletsKit, Networks } = await import(
    "@creit.tech/stellar-wallets-kit"
  );
  const { defaultModules } = await import(
    "@creit.tech/stellar-wallets-kit/modules/utils"
  );

  StellarWalletsKit.init({
    modules: defaultModules(),
    network: networkConfig.network === "mainnet" ? Networks.PUBLIC : Networks.TESTNET,
  });

  _initialized = true;
}

export async function kitConnect(): Promise<string> {
  await initWalletKit();
  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit");
  const { address } = await StellarWalletsKit.authModal();
  return address;
}

export async function kitGetAddress(): Promise<string> {
  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit");
  const { address } = await StellarWalletsKit.getAddress();
  return address;
}

export async function kitSign(xdr: string): Promise<string> {
  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit");
  const { signedTxXdr } = await StellarWalletsKit.signTransaction(xdr, {
    networkPassphrase: networkConfig.networkPassphrase,
  });
  return signedTxXdr;
}

export async function kitDisconnect(): Promise<void> {
  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit");
  await StellarWalletsKit.disconnect();
}

export async function kitGetNetwork(): Promise<string> {
  const { StellarWalletsKit } = await import("@creit.tech/stellar-wallets-kit");
  const result = await StellarWalletsKit.getNetwork();
  return result.networkPassphrase;
}

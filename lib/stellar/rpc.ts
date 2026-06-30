import * as StellarSdk from "@stellar/stellar-sdk";
import { networkConfig } from "./config";

let _rpc: StellarSdk.rpc.Server | null = null;

export function getRpc(): StellarSdk.rpc.Server {
  if (!_rpc) {
    _rpc = new StellarSdk.rpc.Server(networkConfig.rpcUrl);
  }
  return _rpc;
}

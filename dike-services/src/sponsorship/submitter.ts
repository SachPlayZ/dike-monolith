import * as StellarSdk from "@stellar/stellar-sdk";
import { SponsorshipError } from "./types.js";

export interface SponsorshipRpc {
  sendTransaction(transaction: StellarSdk.FeeBumpTransaction): Promise<{
    status: string;
    hash?: string;
    errorResult?: { toXDR(format: "base64"): string };
  }>;
  getTransaction(hash: string): Promise<{
    status: string;
    returnValue?: StellarSdk.xdr.ScVal;
  }>;
}

export interface SubmittedSponsoredTransaction {
  outerHash: string;
  returnValueXdr?: string;
}

export class RpcSponsorshipSubmitter {
  constructor(
    private readonly rpc: SponsorshipRpc,
    private readonly options: { timeoutSeconds: number; pollIntervalMs?: number } = { timeoutSeconds: 120 },
    private readonly sleep: (ms: number) => Promise<void> = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
  ) {}

  async submit(transaction: StellarSdk.FeeBumpTransaction): Promise<SubmittedSponsoredTransaction> {
    const sent = await this.rpc.sendTransaction(transaction);
    if (sent.status !== "PENDING" || !sent.hash) {
      throw new SponsorshipError("RPC_REJECTED", "Stellar RPC rejected the sponsored transaction.", { retryable: true });
    }

    const deadline = Date.now() + this.options.timeoutSeconds * 1000;
    let response = await this.rpc.getTransaction(sent.hash);
    while (response.status === "NOT_FOUND" && Date.now() < deadline) {
      await this.sleep(this.options.pollIntervalMs ?? 1_000);
      response = await this.rpc.getTransaction(sent.hash);
    }

    if (response.status === "SUCCESS") {
      return {
        outerHash: sent.hash,
        ...(response.returnValue ? { returnValueXdr: response.returnValue.toXDR("base64") } : {}),
      };
    }
    if (response.status === "FAILED") {
      throw new SponsorshipError("TRANSACTION_FAILED", "The sponsored transaction failed on-chain.");
    }
    throw new SponsorshipError("CONFIRMATION_TIMEOUT", "The sponsored transaction was not confirmed in time.", { retryable: true });
  }
}

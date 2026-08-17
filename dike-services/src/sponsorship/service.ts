import type { DikeManifestContracts } from "../config/manifest.js";
import { assertAllowedContractCall } from "./allowlist.js";
import { buildAndSignFeeBump } from "./fee-bump.js";
import { quoteSponsoredFee, type FeePolicy } from "./fees.js";
import { parseSponsorshipRequest } from "./request.js";
import { RedisSponsorshipReplay, type ReplayRecord } from "./replay.js";
import type { SponsorSigner } from "./signer.js";
import type { RpcSponsorshipSubmitter } from "./submitter.js";
import { SponsorshipError, type SponsorshipResult } from "./types.js";
import { verifyInnerSourceSignature } from "./signature.js";
import { parseSponsoredInnerTransaction } from "./validator.js";
import type { RedisSponsorshipQuota, BudgetReservation } from "./quota.js";

export interface FeeSponsorshipServiceOptions {
  enabled: boolean;
  networkPassphrase: string;
  contracts: DikeManifestContracts;
  feePolicy: FeePolicy;
  signer: SponsorSigner;
  quota: RedisSponsorshipQuota;
  replay: RedisSponsorshipReplay;
  submitter: RpcSponsorshipSubmitter;
}

export class FeeSponsorshipService {
  constructor(private readonly options: FeeSponsorshipServiceOptions) {}

  async sponsor(body: unknown, ip: string): Promise<SponsorshipResult> {
    if (!this.options.enabled) {
      throw new SponsorshipError("SPONSORSHIP_DISABLED", "Fee sponsorship is not enabled.");
    }

    const request = parseSponsorshipRequest(body);
    const parsed = parseSponsoredInnerTransaction(
      request.signedTransactionXdr,
      this.options.networkPassphrase,
    );
    verifyInnerSourceSignature(parsed);
    assertAllowedContractCall(parsed, this.options.contracts);
    const quote = quoteSponsoredFee(parsed, this.options.feePolicy);

    const terminal = await this.options.replay.terminal(parsed.hash);
    if (terminal) return this.replayResult(terminal);
    const lockToken = await this.options.replay.acquire(parsed.hash);
    let reservation: BudgetReservation | null = null;
    let submissionStarted = false;

    try {
      reservation = await this.options.quota.reserve(parsed.source, ip, quote.outerFee);
      const built = buildAndSignFeeBump(
        parsed,
        this.options.feePolicy,
        this.options.signer,
        this.options.networkPassphrase,
      );
      submissionStarted = true;
      const submitted = await this.options.submitter.submit(built.transaction);
      const result: SponsorshipResult = {
        innerHash: parsed.hash,
        outerHash: submitted.outerHash,
        status: "SUCCESS",
        ...(submitted.returnValueXdr ? { returnValueXdr: submitted.returnValueXdr } : {}),
      };
      await this.options.replay.complete(parsed.hash, lockToken, { status: "success", result });
      return result;
    } catch (error) {
      const safeCode = error instanceof SponsorshipError ? error.code : "RPC_REJECTED";
      if (reservation && !submissionStarted) await this.options.quota.releaseBudget(reservation);
      if (submissionStarted) {
        await this.options.replay.complete(parsed.hash, lockToken, { status: "failure", code: safeCode });
      } else {
        await this.options.replay.release(parsed.hash, lockToken);
      }
      throw error;
    }
  }

  private replayResult(record: ReplayRecord): SponsorshipResult {
    if (record.status === "success") return record.result;
    throw new SponsorshipError(record.code, "This sponsored transaction already reached a terminal state.");
  }
}

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
import type { MetricsStore } from "../observability/metrics.js";
import type { Logger } from "../observability/logger.js";

export interface FeeSponsorshipServiceOptions {
  enabled: boolean;
  networkPassphrase: string;
  contracts: DikeManifestContracts;
  feePolicy: FeePolicy;
  signer: SponsorSigner;
  quota: RedisSponsorshipQuota;
  replay: RedisSponsorshipReplay;
  submitter: RpcSponsorshipSubmitter;
  metrics?: MetricsStore;
  logger?: Logger;
}

export class FeeSponsorshipService {
  constructor(private readonly options: FeeSponsorshipServiceOptions) {}

  async sponsor(body: unknown, ip: string): Promise<SponsorshipResult> {
    this.options.metrics?.noteSponsorshipRequested();
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
    this.options.metrics?.noteSponsorshipAccepted(quote.outerFee);
    const startedAt = Date.now();

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
      this.options.metrics?.noteSponsorshipCompleted("confirmed", Date.now() - startedAt);
      this.options.logger?.info({ event: "fee_sponsorship", innerHash: parsed.hash, outerHash: result.outerHash, source: parsed.source, method: parsed.method, outcome: "confirmed" }, "fee sponsorship completed");
      return result;
    } catch (error) {
      const safeCode = error instanceof SponsorshipError ? error.code : "RPC_REJECTED";
      const outcome = safeCode === "CONFIRMATION_TIMEOUT"
        ? "timeout"
        : safeCode === "TRANSACTION_FAILED"
          ? "failed"
          : "rejected";
      this.options.metrics?.noteSponsorshipCompleted(outcome, Date.now() - startedAt, safeCode);
      this.options.logger?.warn({ event: "fee_sponsorship", innerHash: parsed.hash, source: parsed.source, method: parsed.method, outcome, code: safeCode }, "fee sponsorship failed");
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

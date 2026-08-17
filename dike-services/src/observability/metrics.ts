type ContractMetric = {
  contractId: string;
  module: string;
  lastIndexedLedger: number;
  latestLedger: number;
  lagLedgers: number;
};

export class MetricsStore {
  private latestLedger = 0;
  private rpcFailureCount = 0;
  private reconciliationMismatchCount = 0;
  private persistentReconciliationMismatchCount = 0;
  private processingFailures = 0;
  private lagAlertCount = 0;
  private checkpointRewindCount = 0;
  private sponsorshipRequested = 0;
  private sponsorshipAccepted = 0;
  private sponsorshipConfirmed = 0;
  private sponsorshipFailed = 0;
  private sponsorshipTimeout = 0;
  private sponsorshipRejected = 0;
  private sponsorshipDeclaredStroops = 0n;
  private sponsorshipLatencyMs = 0;
  private sponsorshipRejectionReasons = new Map<string, number>();
  private contractMetrics = new Map<string, ContractMetric>();

  setLatestLedger(sequence: number) {
    this.latestLedger = sequence;
  }

  noteRpcFailure() {
    this.rpcFailureCount += 1;
  }

  noteProcessingFailure() {
    this.processingFailures += 1;
  }

  setReconciliationMismatchCount(count: number) {
    this.reconciliationMismatchCount = count;
  }

  setPersistentReconciliationMismatchCount(count: number) {
    this.persistentReconciliationMismatchCount = count;
  }

  setLagAlertCount(count: number) {
    this.lagAlertCount = count;
  }

  noteCheckpointRewind() {
    this.checkpointRewindCount += 1;
  }

  noteSponsorshipRequested() {
    this.sponsorshipRequested += 1;
  }

  noteSponsorshipAccepted(declaredStroops: bigint) {
    this.sponsorshipAccepted += 1;
    this.sponsorshipDeclaredStroops += declaredStroops;
  }

  noteSponsorshipCompleted(
    status: "confirmed" | "failed" | "timeout" | "rejected",
    latencyMs: number,
    reason?: string,
  ) {
    this.sponsorshipLatencyMs += Math.max(latencyMs, 0);
    if (status === "confirmed") this.sponsorshipConfirmed += 1;
    if (status === "failed") this.sponsorshipFailed += 1;
    if (status === "timeout") this.sponsorshipTimeout += 1;
    if (status === "rejected") this.sponsorshipRejected += 1;
    if (reason) this.sponsorshipRejectionReasons.set(reason, (this.sponsorshipRejectionReasons.get(reason) ?? 0) + 1);
  }

  setContractLag(contractId: string, module: string, lastIndexedLedger: number, latestLedger: number) {
    this.contractMetrics.set(contractId, {
      contractId,
      module,
      lastIndexedLedger,
      latestLedger,
      lagLedgers: Math.max(latestLedger - lastIndexedLedger, 0),
    });
  }

  snapshot() {
    return {
      latestLedger: this.latestLedger,
      rpcFailureCount: this.rpcFailureCount,
      reconciliationMismatchCount: this.reconciliationMismatchCount,
      persistentReconciliationMismatchCount: this.persistentReconciliationMismatchCount,
      processingFailures: this.processingFailures,
      lagAlertCount: this.lagAlertCount,
      checkpointRewindCount: this.checkpointRewindCount,
      sponsorship: {
        requested: this.sponsorshipRequested,
        accepted: this.sponsorshipAccepted,
        confirmed: this.sponsorshipConfirmed,
        failed: this.sponsorshipFailed,
        timeout: this.sponsorshipTimeout,
        rejected: this.sponsorshipRejected,
        declaredStroops: this.sponsorshipDeclaredStroops.toString(),
        latencyMs: this.sponsorshipLatencyMs,
        rejectionReasons: Object.fromEntries(this.sponsorshipRejectionReasons),
      },
      contracts: [...this.contractMetrics.values()],
    };
  }
}

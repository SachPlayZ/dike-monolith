import type { TxState } from "@/lib/types";
import {
  decodeSponsoredReturnValue,
  getSponsorshipStatus,
  submitSponsoredTransaction,
  SponsorshipApiError,
} from "@/lib/api/sponsorship";
import { parseDikeError, submitAndPoll, type TransactionFeeMode } from "./transaction";

export interface TransactionExecutorOptions {
  build: (mode?: TransactionFeeMode) => Promise<string>;
  sign: (xdr: string, options?: { sponsored?: boolean; method?: string }) => Promise<string>;
  method?: string;
  onState?: (state: TxState) => void;
}

export interface ExecutedTransaction {
  hash: string;
  returnValue?: ReturnType<typeof decodeSponsoredReturnValue>;
  sponsored: boolean;
}

function update(onState: TransactionExecutorOptions["onState"], state: TxState) {
  onState?.(state);
}

async function selectMode(): Promise<TransactionFeeMode> {
  const status = await getSponsorshipStatus();
  if (status.available) return "sponsored";
  if (!status.enabled && status.reason === "disabled") return "direct";
  throw new SponsorshipApiError(
    "SPONSORSHIP_UNAVAILABLE",
    "Fee sponsorship is temporarily unavailable. Try again later.",
    true,
    503,
  );
}

export async function executeTransaction(options: TransactionExecutorOptions): Promise<ExecutedTransaction> {
  const { build, sign, method, onState } = options;
  try {
    const mode = await selectMode();
    update(onState, { status: "building", hash: null, error: null });
    const xdr = await build(mode);
    update(onState, { status: "awaiting-signature", hash: null, error: null });
    const signedXdr = await sign(xdr, { sponsored: mode === "sponsored", method });

    if (mode === "sponsored") {
      update(onState, { status: "sponsoring", hash: null, error: null });
      const result = await submitSponsoredTransaction(signedXdr);
      update(onState, { status: "pending", hash: result.outerHash, error: null });
      return {
        hash: result.outerHash,
        returnValue: decodeSponsoredReturnValue(result.returnValueXdr),
        sponsored: true,
      };
    }

    update(onState, { status: "submitting", hash: null, error: null });
    const result = await submitAndPoll(signedXdr);
    update(onState, { status: "pending", hash: result.hash, error: null });
    return { hash: result.hash, returnValue: result.returnValue, sponsored: false };
  } catch (error) {
    update(onState, { status: "failed", hash: null, error: parseDikeError(error) });
    throw error;
  }
}

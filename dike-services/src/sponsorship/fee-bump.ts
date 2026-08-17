import * as StellarSdk from "@stellar/stellar-sdk";
import type { SponsorSigner } from "./signer.js";
import { quoteSponsoredFee, type FeePolicy, type FeeQuote } from "./fees.js";
import { SponsorshipError, type ParsedSponsorshipTransaction } from "./types.js";

export interface BuiltFeeBump {
  transaction: StellarSdk.FeeBumpTransaction;
  xdr: string;
  hash: string;
  quote: FeeQuote;
}

export function buildAndSignFeeBump(
  parsed: ParsedSponsorshipTransaction,
  policy: FeePolicy,
  signer: SponsorSigner,
  networkPassphrase: string,
): BuiltFeeBump {
  const quote = quoteSponsoredFee(parsed, policy);
  const transaction = StellarSdk.TransactionBuilder.buildFeeBumpTransaction(
    signer.publicKey(),
    policy.baseFeeStroops,
    parsed.transaction,
    networkPassphrase,
  );

  if (BigInt(transaction.fee) !== quote.outerFee) {
    throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The fee-bump fee did not match the sponsor policy.");
  }
  signer.sign(transaction);

  return {
    transaction,
    xdr: transaction.toXDR(),
    hash: transaction.hash().toString("hex"),
    quote,
  };
}

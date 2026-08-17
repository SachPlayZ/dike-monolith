import * as StellarSdk from "@stellar/stellar-sdk";
import { SponsorshipError, type ParsedSponsorshipTransaction } from "./types.js";

const MAX_VALIDITY_SECONDS = 15 * 60;

function sorobanData(transaction: StellarSdk.Transaction) {
  try {
    return transaction.toEnvelope().v1().tx().ext().sorobanData();
  } catch {
    return null;
  }
}

export function parseSponsoredInnerTransaction(
  xdr: string,
  networkPassphrase: string,
  nowSeconds = Math.floor(Date.now() / 1000),
  maxValiditySeconds = MAX_VALIDITY_SECONDS,
): ParsedSponsorshipTransaction {
  let transaction: StellarSdk.Transaction;
  try {
    const parsed = StellarSdk.TransactionBuilder.fromXDR(xdr, networkPassphrase);
    if (!(parsed instanceof StellarSdk.Transaction)) {
      throw new SponsorshipError("UNSUPPORTED_ENVELOPE", "Fee bumps cannot be nested.");
    }
    transaction = parsed;
  } catch (error) {
    if (error instanceof SponsorshipError) throw error;
    throw new SponsorshipError("MALFORMED_XDR", "Could not decode the signed transaction.", { cause: error });
  }

  if (!StellarSdk.StrKey.isValidEd25519PublicKey(transaction.source)) {
    throw new SponsorshipError("UNSUPPORTED_SOURCE", "The inner source must be a standard G account.");
  }
  if (transaction.signatures.length === 0) {
    throw new SponsorshipError("BAD_SIGNATURE", "The inner transaction has no source signature.");
  }

  const timeBounds = transaction.timeBounds;
  if (!timeBounds || Number(timeBounds.maxTime) <= nowSeconds) {
    throw new SponsorshipError("EXPIRED_TRANSACTION", "The signed transaction has expired.");
  }
  if (Number(timeBounds.maxTime) > nowSeconds + maxValiditySeconds) {
    throw new SponsorshipError("EXPIRED_TRANSACTION", "The signed transaction is valid for too long.");
  }

  const operation = transaction.operations[0];
  if (transaction.operations.length !== 1 || !operation || operation.type !== "invokeHostFunction") {
    throw new SponsorshipError("UNSUPPORTED_SOURCE", "Only one Soroban contract operation may be sponsored.");
  }
  if (operation.func.switch().name !== "hostFunctionTypeInvokeContract") {
    throw new SponsorshipError("UNSUPPORTED_ENVELOPE", "Only contract invocations may be sponsored.");
  }

  const invoke = operation.func.invokeContract();
  if (invoke.contractAddress().switch().name !== "scAddressTypeContract") {
    throw new SponsorshipError("DISALLOWED_CONTRACT", "The invocation must target a contract address.");
  }

  const data = sorobanData(transaction);
  if (!data) {
    throw new SponsorshipError("UNSUPPORTED_ENVELOPE", "The Soroban transaction is not assembled.");
  }

  const resourceFee = BigInt(data.resourceFee().toString());
  const innerFee = BigInt(transaction.fee);
  if (resourceFee < 0n || innerFee < resourceFee) {
    throw new SponsorshipError("FEE_LIMIT_EXCEEDED", "The signed transaction has invalid fee data.");
  }

  const contractId = StellarSdk.StrKey.encodeContract(
    Buffer.from(invoke.contractAddress().contractId() as unknown as ArrayLike<number>),
  );
  const method = invoke.functionName().toString();

  return {
    xdr,
    transaction,
    source: transaction.source,
    hash: transaction.hash().toString("hex"),
    contractId,
    method,
    innerFee,
    resourceFee,
  };
}

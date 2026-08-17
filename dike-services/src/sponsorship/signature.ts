import { timingSafeEqual } from "node:crypto";
import * as StellarSdk from "@stellar/stellar-sdk";
import { SponsorshipError, type ParsedSponsorshipTransaction } from "./types.js";

export function verifyInnerSourceSignature(parsed: ParsedSponsorshipTransaction) {
  const signatures = parsed.transaction.signatures;
  if (signatures.length !== 1) {
    throw new SponsorshipError("BAD_SIGNATURE", "Exactly one source signature is required.");
  }

  const signature = signatures[0];
  if (!signature) {
    throw new SponsorshipError("BAD_SIGNATURE", "The source signature is missing.");
  }

  try {
    const publicKey = StellarSdk.StrKey.decodeEd25519PublicKey(parsed.source);
    const expectedHint = publicKey.subarray(publicKey.length - 4);
    const actualHint = Buffer.from(signature.hint());
    if (actualHint.length !== expectedHint.length || !timingSafeEqual(actualHint, expectedHint)) {
      throw new SponsorshipError("BAD_SIGNATURE", "The source signature hint does not match.");
    }

    if (!StellarSdk.Keypair.fromPublicKey(parsed.source).verify(parsed.transaction.hash(), signature.signature())) {
      throw new SponsorshipError("BAD_SIGNATURE", "The source signature is invalid.");
    }
  } catch (error) {
    if (error instanceof SponsorshipError) throw error;
    throw new SponsorshipError("BAD_SIGNATURE", "The source signature is invalid.", { cause: error });
  }
}

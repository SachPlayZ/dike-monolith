import * as StellarSdk from "@stellar/stellar-sdk";

export interface SponsorSigner {
  publicKey(): string;
  sign(transaction: StellarSdk.FeeBumpTransaction): void;
}

/**
 * Runtime seed signer. Keep this adapter small so a KMS/HSM signer can replace
 * it without changing fee-bump validation or submission.
 */
export function createSeedSponsorSigner(seed: string): SponsorSigner {
  const keypair = StellarSdk.Keypair.fromSecret(seed);

  return {
    publicKey: () => keypair.publicKey(),
    sign: (transaction) => transaction.sign(keypair),
  };
}

import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { createSeedSponsorSigner } from "./signer.js";

describe("createSeedSponsorSigner", () => {
  it("derives the public key and signs a fee-bump envelope", () => {
    const keypair = StellarSdk.Keypair.random();
    const signer = createSeedSponsorSigner(keypair.secret());
    const inner = new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(keypair.publicKey(), "1"),
      {
        fee: StellarSdk.BASE_FEE,
        networkPassphrase: StellarSdk.Networks.TESTNET,
      },
    )
      .addOperation(
        StellarSdk.Operation.payment({
          destination: StellarSdk.Keypair.random().publicKey(),
          asset: StellarSdk.Asset.native(),
          amount: "1",
        }),
      )
      .setTimeout(60)
      .build();
    inner.sign(keypair);
    const bump = StellarSdk.TransactionBuilder.buildFeeBumpTransaction(
      keypair.publicKey(),
      StellarSdk.BASE_FEE,
      inner,
      StellarSdk.Networks.TESTNET,
    );

    signer.sign(bump);

    expect(signer.publicKey()).toBe(keypair.publicKey());
    expect(bump.signatures).toHaveLength(1);
  });

  it("rejects malformed runtime seed material", () => {
    expect(() => createSeedSponsorSigner("not-a-stellar-seed")).toThrow();
  });
});

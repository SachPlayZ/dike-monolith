import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { buildAndSignFeeBump } from "./fee-bump.js";
import { createSeedSponsorSigner } from "./signer.js";
import { parseSponsoredInnerTransaction } from "./validator.js";

describe("buildAndSignFeeBump", () => {
  it("keeps the signed inner transaction and signs the outer envelope", () => {
    const source = StellarSdk.Keypair.random();
    const sponsor = StellarSdk.Keypair.random();
    const contractId = StellarSdk.StrKey.encodeContract(Buffer.from("contract-id".padEnd(32, "0")));
    const transaction = new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(source.publicKey(), "1"),
      {
        fee: "2000000",
        networkPassphrase: StellarSdk.Networks.TESTNET,
        sorobanData: new StellarSdk.SorobanDataBuilder().setResources(10_000, 100, 100).setResourceFee("1000").build(),
      },
    )
      .addOperation(new StellarSdk.Contract(contractId).call("buy_yes"))
      .setTimeout(300)
      .build();
    transaction.sign(source);
    const parsed = parseSponsoredInnerTransaction(transaction.toXDR(), StellarSdk.Networks.TESTNET);
    const built = buildAndSignFeeBump(
      parsed,
      { baseFeeStroops: "2000000", maxTotalFeeStroops: "10000000", maxResourceFeeStroops: "8000000" },
      createSeedSponsorSigner(sponsor.secret()),
      StellarSdk.Networks.TESTNET,
    );

    const decoded = StellarSdk.TransactionBuilder.fromXDR(built.xdr, StellarSdk.Networks.TESTNET);
    expect(decoded).toBeInstanceOf(StellarSdk.FeeBumpTransaction);
    const bump = decoded as StellarSdk.FeeBumpTransaction;
    expect(bump.fee).toBe("4001000");
    expect(bump.feeSource).toBe(sponsor.publicKey());
    expect(bump.signatures).toHaveLength(1);
    expect(bump.innerTransaction.signatures).toHaveLength(1);
    expect(bump.innerTransaction.hash().toString("hex")).toBe(parsed.hash);
  });
});

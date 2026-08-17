import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { parseSponsoredInnerTransaction } from "./validator.js";
import { verifyInnerSourceSignature } from "./signature.js";
import { SponsorshipError } from "./types.js";

function makeTransaction(method: string, sequence = "1") {
  const source = StellarSdk.Keypair.random();
  const signer = StellarSdk.Keypair.random();
  const contractId = StellarSdk.StrKey.encodeContract(Buffer.from("contract-id".padEnd(32, "0")));
  const data = new StellarSdk.SorobanDataBuilder()
    .setResources(10_000, 100, 100)
    .setResourceFee("1000")
    .build();
  const transaction = new StellarSdk.TransactionBuilder(
    new StellarSdk.Account(source.publicKey(), sequence),
    {
      fee: "1100",
      networkPassphrase: StellarSdk.Networks.TESTNET,
      sorobanData: data,
    },
  )
    .addOperation(new StellarSdk.Contract(contractId).call(method))
    .setTimeout(300)
    .build();
  transaction.sign(signer);
  return { source, signer, transaction };
}

describe("verifyInnerSourceSignature", () => {
  it("accepts a valid source signature", () => {
    const { source, transaction } = makeTransaction("buy_yes");
    const parsed = parseSponsoredInnerTransaction(transaction.toXDR(), StellarSdk.Networks.TESTNET);
    expect(() => verifyInnerSourceSignature(parsed)).toThrow(SponsorshipError);

    const valid = new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(source.publicKey(), "1"),
      {
        fee: "1100",
        networkPassphrase: StellarSdk.Networks.TESTNET,
        sorobanData: new StellarSdk.SorobanDataBuilder().setResources(10_000, 100, 100).setResourceFee("1000").build(),
      },
    )
      .addOperation(new StellarSdk.Contract(parsed.contractId).call("buy_yes"))
      .setTimeout(300)
      .build();
    valid.sign(source);
    const validParsed = parseSponsoredInnerTransaction(valid.toXDR(), StellarSdk.Networks.TESTNET);
    expect(() => verifyInnerSourceSignature(validParsed)).not.toThrow();
  });

  it("rejects signatures from another key and multiple signatures", () => {
    const { transaction } = makeTransaction("buy_yes");
    const parsed = parseSponsoredInnerTransaction(transaction.toXDR(), StellarSdk.Networks.TESTNET);
    expect(() => verifyInnerSourceSignature(parsed)).toThrow(SponsorshipError);

    const source = StellarSdk.Keypair.random();
    const data = new StellarSdk.SorobanDataBuilder().setResources(10_000, 100, 100).setResourceFee("1000").build();
    const valid = new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(source.publicKey(), "1"),
      { fee: "1100", networkPassphrase: StellarSdk.Networks.TESTNET, sorobanData: data },
    )
      .addOperation(new StellarSdk.Contract(parsed.contractId).call("buy_yes"))
      .setTimeout(300)
      .build();
    valid.sign(source);
    const duplicate = valid.signatures[0];
    if (!duplicate) throw new Error("expected source signature");
    valid.addDecoratedSignature(duplicate);
    const multi = parseSponsoredInnerTransaction(valid.toXDR(), StellarSdk.Networks.TESTNET);
    expect(() => verifyInnerSourceSignature(multi)).toThrow(SponsorshipError);
  });
});

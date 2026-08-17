import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { parseSponsoredInnerTransaction } from "./validator.js";
import { SponsorshipError } from "./types.js";

function makeTransaction() {
  const source = StellarSdk.Keypair.random();
  const contractId = StellarSdk.StrKey.encodeContract(Buffer.from(StellarSdk.hash(Buffer.from("contract"))));
  const data = new StellarSdk.SorobanDataBuilder()
    .setResources(10_000, 100, 100)
    .setResourceFee("1000")
    .build();
  const transaction = new StellarSdk.TransactionBuilder(
    new StellarSdk.Account(source.publicKey(), "1"),
    {
      fee: "1100",
      networkPassphrase: StellarSdk.Networks.TESTNET,
      sorobanData: data,
    },
  )
    .addOperation(new StellarSdk.Contract(contractId).call("buy_yes"))
    .setTimeout(300)
    .build();
  transaction.sign(source);
  return { source, transaction };
}

describe("parseSponsoredInnerTransaction", () => {
  it("extracts source, contract, method, and resource fee", () => {
    const { source, transaction } = makeTransaction();
    const parsed = parseSponsoredInnerTransaction(
      transaction.toXDR(),
      StellarSdk.Networks.TESTNET,
    );
    expect(parsed.source).toBe(source.publicKey());
    expect(parsed.contractId).toMatch(/^C/);
    expect(parsed.method).toBe("buy_yes");
    expect(parsed.resourceFee).toBe(1000n);
    expect(parsed.innerFee).toBe(2100n);
  });

  it("rejects unsigned, classic, and nested fee-bump envelopes", () => {
    const { transaction, source } = makeTransaction();
    const unsigned = StellarSdk.TransactionBuilder.fromXDR(
      transaction.toXDR(),
      StellarSdk.Networks.TESTNET,
    ) as StellarSdk.Transaction;
    unsigned.signatures.splice(0, unsigned.signatures.length);
    expect(() => parseSponsoredInnerTransaction(unsigned.toXDR(), StellarSdk.Networks.TESTNET))
      .toThrow(SponsorshipError);

    const payment = new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(source.publicKey(), "2"),
      { fee: StellarSdk.BASE_FEE, networkPassphrase: StellarSdk.Networks.TESTNET },
    )
      .addOperation(StellarSdk.Operation.payment({
        destination: StellarSdk.Keypair.random().publicKey(),
        asset: StellarSdk.Asset.native(),
        amount: "1",
      }))
      .setTimeout(300)
      .build();
    payment.sign(source);
    expect(() => parseSponsoredInnerTransaction(payment.toXDR(), StellarSdk.Networks.TESTNET))
      .toThrow(SponsorshipError);
  });
});

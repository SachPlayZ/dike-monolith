import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import {
  feeFromXdr,
  parseDikeError,
  sponsorMaximumFeeFromXdr,
  walletFeeFromXdr,
} from "@/lib/stellar/transaction";

describe("parseDikeError", () => {
  it("maps known contract error codes", () => {
    expect(parseDikeError(new Error("Error(Contract, #17)"))).toBe("SlippageExceeded");
    expect(parseDikeError(new Error("Error(Contract, #3)"))).toBe("Unauthorized");
  });

  it("decodes raw Stellar XDR failures into readable status text", () => {
    expect(
      parseDikeError(
        new Error(
          "Transaction rejected: AAAAAAAAAGT/////AAAAAQAAAAAAAAAB////+gAAAAA="
        )
      )
    ).toContain("txFailed");
  });
});

describe("transaction fee helpers", () => {
  it("distinguishes wallet and sponsor fees", () => {
    const source = StellarSdk.Keypair.random();
    const tx = new StellarSdk.TransactionBuilder(new StellarSdk.Account(source.publicKey(), "1"), {
      fee: "2000000",
      networkPassphrase: StellarSdk.Networks.TESTNET,
    })
      .addOperation(StellarSdk.Operation.payment({
        destination: StellarSdk.Keypair.random().publicKey(),
        asset: StellarSdk.Asset.native(),
        amount: "1",
      }))
      .setTimeout(60)
      .build();
    const xdr = tx.toXDR();
    expect(feeFromXdr(xdr)).toBe("2000000");
    expect(walletFeeFromXdr(xdr, false)).toBe("2000000");
    expect(walletFeeFromXdr(xdr, true)).toBe("0");
    expect(sponsorMaximumFeeFromXdr(xdr)).toBe("200");
  });
});

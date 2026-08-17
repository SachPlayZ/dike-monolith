import { describe, expect, it } from "vitest";
import { parseDikeError } from "@/lib/stellar/transaction";

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

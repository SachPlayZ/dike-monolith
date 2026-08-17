import { describe, expect, it } from "vitest";
import { applySlippage, formatUsdc, parseUsdc } from "@/lib/stellar/scval";

describe("scval helpers", () => {
  it("round-trips USDC formatting and parsing", () => {
    const raw = 12_345_678n;
    expect(formatUsdc(raw)).toBe("1.2345678");
    expect(parseUsdc("1.2345678")).toBe(raw);
    expect(parseUsdc(formatUsdc(90_000_000n))).toBe(90_000_000n);
  });

  it("applies slippage in basis points", () => {
    expect(applySlippage(1_000_000n, 50)).toBe(995_000n);
    expect(applySlippage(123_456_789n, 100)).toBe(122_222_221n);
  });
});

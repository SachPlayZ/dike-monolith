import { describe, expect, it } from "vitest";
import { getReferenceUrlError, safeReferenceUrl } from "./reference-url";

describe("reference URL validation", () => {
  it("accepts public HTTPS URLs", () => {
    expect(getReferenceUrlError("https://docs.dikeprotocol.xyz/market-7")).toBeNull();
    expect(safeReferenceUrl(" https://docs.dikeprotocol.xyz/market-7 ")).toBe(
      "https://docs.dikeprotocol.xyz/market-7",
    );
  });

  it.each([
    "javascript:alert(1)",
    "http://example.com/rules",
    "https://example.com/rules",
    "https://localhost/rules",
    "https://127.0.0.1/rules",
    "https://user:pass@rules.example.org/rules",
  ])("rejects unsafe or placeholder URL %s", (value) => {
    expect(getReferenceUrlError(value)).not.toBeNull();
    expect(safeReferenceUrl(value)).toBeNull();
  });
});

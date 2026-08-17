import { describe, expect, it } from "vitest";
import { assertAllowedContractCall, SPONSORED_METHODS } from "./allowlist.js";
import { SponsorshipError } from "./types.js";

const contracts = Object.fromEntries(
  Object.keys(SPONSORED_METHODS).map((name, index) => [name, `C${String(index).padStart(55, "0")}`]),
) as typeof import("../config/manifest.js").DikeManifestContracts;

describe("sponsored contract allowlist", () => {
  it("accepts every checked-in application write method", () => {
    for (const [module, methods] of Object.entries(SPONSORED_METHODS)) {
      for (const method of methods) {
        expect(() => assertAllowedContractCall({ contractId: contracts[module as keyof typeof contracts], method }, contracts))
          .not.toThrow();
      }
    }
  });

  it("rejects unknown contracts and methods", () => {
    expect(() => assertAllowedContractCall({ contractId: "Cunknown", method: "buy_yes" }, contracts))
      .toThrow(SponsorshipError);
    expect(() => assertAllowedContractCall({ contractId: contracts.amm, method: "admin_withdraw" }, contracts))
      .toThrow(SponsorshipError);
  });
});

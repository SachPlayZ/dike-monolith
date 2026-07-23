import { describe, expect, it } from "vitest";
import { normalizeMarketData, normalizePortfolio } from "./normalizers";
import { NETWORK } from "@/lib/stellar/config";

const market = normalizeMarketData({
  market_id: "7",
  question: "Will the test pass?",
  status: "Live",
  pool_id: "70",
  tradeable: false,
});

describe("normalizeMarketData", () => {
  it("preserves service-calculated tradeability", () => {
    expect(market.tradeable).toBe(false);
  });

  it("rejects indexed data from another Stellar network", () => {
    const otherNetwork = NETWORK === "mainnet" ? "testnet" : "mainnet";
    expect(() => normalizeMarketData({ market_id: "8", network: otherNetwork })).toThrow(
      new RegExp(`serving "${otherNetwork}" data`),
    );
  });
});

describe("normalizePortfolio", () => {
  it("preserves YES and NO vault accounting independently", () => {
    const [position] = normalizePortfolio(
      {
        positions: [
          { market_id: "7", outcome: "Yes", balance: "11" },
          { market_id: "7", outcome: "No", balance: "22" },
        ],
        lpPositions: [{ pool_id: "70", shares: "33" }],
        vaultState: [{
          market_id: "7",
          user_deposit: "44",
          root_stake_yes: "55",
          root_stake_no: "66",
          child_used_total: "77",
          child_debt: "88",
          parent_debt_yes: "99",
          parent_debt_no: "111",
          redeemed_yes: "122",
          redeemed_no: "133",
        }],
      },
      [market],
    );

    expect(position).toMatchObject({
      yesBalance: "11",
      noBalance: "22",
      lpShares: "33",
      rootStakeYes: "55",
      rootStakeNo: "66",
      parentDebtYes: "99",
      parentDebtNo: "111",
      redeemedYes: "122",
      redeemedNo: "133",
    });
  });

  it("does not drop token-only positions without a vault row", () => {
    const positions = normalizePortfolio(
      {
        positions: [{ market_id: "7", outcome: "No", balance: "25" }],
        lpPositions: [],
        vaultState: [],
      },
      [market],
    );

    expect(positions).toHaveLength(1);
    expect(positions[0]).toMatchObject({ marketId: "7", yesBalance: "0", noBalance: "25" });
  });
});

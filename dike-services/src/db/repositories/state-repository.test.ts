import { describe, expect, it, vi } from "vitest";
import { StateRepository } from "./state-repository.js";

describe("StateRepository", () => {
  it("serializes raw events containing bigint values", async () => {
    const query = vi.fn().mockResolvedValue({
      rows: [],
      rowCount: 1,
    });
    const repository = new StateRepository({ query } as never);

    await repository.recordRawEvent({
      network: "testnet",
      contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
      ledger: 123,
      txHash: "hash",
      eventId: "event-1",
      topic: "mkt_new",
      topicValues: ["mkt_new", 1n],
      payload: {
        amount: 10n,
      },
      rawEvent: {
        ledger: 123,
      },
      cursor: "cursor-1",
    });

    const params = query.mock.calls[0]?.[1] as unknown[];
    expect(params[6]).toBe(JSON.stringify(["mkt_new", "1"]));
    expect(params[7]).toBe(JSON.stringify({ amount: "10" }));
  });

  it("can explicitly reset checkpoints without GREATEST", async () => {
    const query = vi.fn().mockResolvedValue({
      rows: [],
      rowCount: 1,
    });
    const repository = new StateRepository({ query } as never);

    await repository.resetCheckpoint("testnet", "CCONTRACT", 100);

    const sql = query.mock.calls[0]?.[0] as string;
    const params = query.mock.calls[0]?.[1] as unknown[];
    expect(sql).toContain("last_processed_ledger = EXCLUDED.last_processed_ledger");
    expect(sql).not.toContain("GREATEST");
    expect(params).toEqual(["testnet", "CCONTRACT", 100]);
  });

  it("accepts the council vote columns defined by the schema", async () => {
    const query = vi.fn().mockResolvedValue({ rows: [], rowCount: 1 });
    const repository = new StateRepository({ query } as never);

    await repository.upsertCouncilVote({
      network: "testnet",
      case_id: 7,
      voter: "GVOTER",
      has_commit: true,
      has_reveal: true,
      revealed_outcome: "Yes",
      claimed_reward: true,
      correct: true,
      reward_amount: "100",
    });

    const sql = query.mock.calls[0]?.[0] as string;
    expect(sql).toContain("has_commit");
    expect(sql).toContain("has_reveal");
    expect(sql).toContain("claimed_reward");
    expect(sql).toContain("reward_amount");
  });

  it("records LP fee claims idempotently by event id", async () => {
    const query = vi.fn().mockResolvedValue({ rows: [], rowCount: 1 });
    const repository = new StateRepository({ query } as never);

    await repository.addLpFeesClaimed("testnet", 2, "GLP", "25", "event-1");

    const sql = query.mock.calls[0]?.[0] as string;
    expect(sql).toContain("INSERT INTO lp_fee_claim_events");
    expect(sql).toContain("ON CONFLICT (network, event_id) DO NOTHING");
    expect(query.mock.calls[0]?.[1]).toEqual(["testnet", 2, "GLP", "25", "event-1"]);
  });

  it("builds private stats from unique participants and indexed transactions", async () => {
    const query = vi.fn()
      .mockResolvedValueOnce({ rows: [{ count: "3" }], rowCount: 1 })
      .mockResolvedValueOnce({
        rows: [
          {
            tx_hash: "tx-2",
            ledger: "102",
            topics: ["buy", "transfer"],
            event_count: "2",
            created_at: "2026-07-18T10:00:00.000Z",
          },
          {
            tx_hash: "tx-1",
            ledger: "101",
            topics: ["mkt_new"],
            event_count: "1",
            created_at: "2026-07-18T09:00:00.000Z",
          },
        ],
        rowCount: 2,
      });
    const repository = new StateRepository({ query } as never);

    const stats = await repository.getStats("testnet");

    expect(query).toHaveBeenCalledTimes(2);
    expect(query.mock.calls[0]?.[0]).toContain("participant_wallets");
    expect(query.mock.calls[1]?.[0]).toContain("GROUP BY tx_hash");
    expect(query.mock.calls[0]?.[1]).toEqual(["testnet"]);
    expect(query.mock.calls[1]?.[1]).toEqual(["testnet"]);
    expect(stats).toEqual({
      connectedWallets: 83,
      indexedWallets: 3,
      transactionCount: 2,
      transactions: [
        {
          hash: "tx-2",
          ledger: "102",
          topics: ["buy", "transfer"],
          eventCount: 2,
          createdAt: "2026-07-18T10:00:00.000Z",
        },
        {
          hash: "tx-1",
          ledger: "101",
          topics: ["mkt_new"],
          eventCount: 1,
          createdAt: "2026-07-18T09:00:00.000Z",
        },
      ],
    });
  });
});

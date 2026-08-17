import { describe, expect, it } from "vitest";
import * as StellarSdk from "@stellar/stellar-sdk";
import { decodeRawEvent, normalizeContractValue } from "./codecs.js";

describe("normalizeContractValue", () => {
  it("serializes Date instances to ISO strings instead of {}", () => {
    const date = new Date("2026-07-01T19:22:50.722Z");
    expect(normalizeContractValue(date)).toBe("2026-07-01T19:22:50.722Z");
  });

  it("serializes Date fields nested in objects and arrays", () => {
    const date = new Date("2026-07-01T19:22:50.722Z");
    expect(normalizeContractValue({ updated_at: date })).toEqual({
      updated_at: "2026-07-01T19:22:50.722Z",
    });
    expect(normalizeContractValue([{ updated_at: date }])).toEqual([
      { updated_at: "2026-07-01T19:22:50.722Z" },
    ]);
  });

  it("still converts bigint and Buffer as before", () => {
    expect(normalizeContractValue(10n)).toBe("10");
    expect(normalizeContractValue(Buffer.from("ab", "hex"))).toBe("ab");
  });
});

describe("decodeRawEvent", () => {
  it("decodes topics/payload normally when every ScVal is valid", () => {
    const topicSymbol = StellarSdk.nativeToScVal("deposit", { type: "symbol" });
    const payload = StellarSdk.nativeToScVal(42, { type: "u64" });

    const decoded = decodeRawEvent({
      topic: [topicSymbol],
      value: payload,
    } as never);

    expect(decoded.topic).toBe("deposit");
    expect(decoded.topicValues).toEqual(["deposit"]);
    expect(decoded.payload).toBe(42n);
  });

  it("does not throw when one topic entry fails to decode, and yields undefined for it", () => {
    const validTopic = StellarSdk.nativeToScVal("deposit", { type: "symbol" });
    const brokenTopic = "not-valid-base64-xdr";
    const payload = StellarSdk.nativeToScVal(1, { type: "u32" });

    const decoded = decodeRawEvent({
      topic: [validTopic, brokenTopic],
      value: payload,
    } as never);

    expect(decoded.topic).toBe("deposit");
    expect(decoded.topicValues).toEqual(["deposit", undefined]);
    expect(decoded.payload).toBe(1);
  });

  it("does not throw when the payload fails to decode, and yields undefined for it", () => {
    const validTopic = StellarSdk.nativeToScVal("fee", { type: "symbol" });

    const decoded = decodeRawEvent({
      topic: [validTopic],
      value: "not-valid-base64-xdr",
    } as never);

    expect(decoded.topic).toBe("fee");
    expect(decoded.payload).toBeUndefined();
  });
});

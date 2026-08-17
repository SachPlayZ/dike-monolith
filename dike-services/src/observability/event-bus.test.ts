import { describe, expect, it, vi } from "vitest";
import { eventBus } from "./event-bus.js";

describe("eventBus", () => {
  it("delivers published updates to subscribers", () => {
    const listener = vi.fn();
    const unsubscribe = eventBus.subscribe(listener);

    eventBus.publish({ type: "market", network: "testnet", marketId: 7 });

    expect(listener).toHaveBeenCalledWith({ type: "market", network: "testnet", marketId: 7 });
    unsubscribe();
  });

  it("stops delivering updates after unsubscribe", () => {
    const listener = vi.fn();
    const unsubscribe = eventBus.subscribe(listener);
    unsubscribe();

    eventBus.publish({ type: "governance", network: "testnet" });

    expect(listener).not.toHaveBeenCalled();
  });

  it("does not leak listeners across many subscribers", () => {
    const before = eventBus.listenerCount("update");
    const unsubscribers = Array.from({ length: 50 }, () => eventBus.subscribe(() => {}));
    expect(eventBus.listenerCount("update")).toBe(before + 50);
    unsubscribers.forEach((unsub) => unsub());
    expect(eventBus.listenerCount("update")).toBe(before);
  });
});

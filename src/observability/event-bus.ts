import { EventEmitter } from "node:events";

export type StateUpdate =
  | { type: "market"; network: string; marketId: number }
  | { type: "portfolio"; network: string; address: string }
  | { type: "governance"; network: string }
  | { type: "council_case"; network: string; caseId: number }
  | { type: "timelock_action"; network: string; actionId: number };

class StateEventBus extends EventEmitter {
  publish(update: StateUpdate) {
    this.emit("update", update);
  }

  subscribe(listener: (update: StateUpdate) => void): () => void {
    this.on("update", listener);
    return () => this.off("update", listener);
  }
}

// Long-lived SSE connections all subscribe to this single process-wide bus;
// unbounded listener count is expected, not a leak.
export const eventBus = new StateEventBus();
eventBus.setMaxListeners(0);

import * as StellarSdk from "@stellar/stellar-sdk";
import type { Outcome } from "@/lib/types";

export function toAddress(addr: string): StellarSdk.xdr.ScVal {
  return StellarSdk.Address.fromString(addr).toScVal();
}

export function toI128(amount: bigint): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(amount, { type: "i128" });
}

export function toU128(amount: bigint): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(amount, { type: "u128" });
}

export function toU64(n: bigint): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(n, { type: "u64" });
}

export function toU32(n: number): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(n, { type: "u32" });
}

export function toI32(n: number): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(n, { type: "i32" });
}

export function toSymbol(s: string): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(s, { type: "symbol" });
}

export function toString(s: string): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(s, { type: "string" });
}

export function toBool(b: boolean): StellarSdk.xdr.ScVal {
  return StellarSdk.nativeToScVal(b);
}

export function toBytes(hex: string): StellarSdk.xdr.ScVal {
  const buf = Buffer.from(hex.replace(/^0x/, ""), "hex");
  return StellarSdk.nativeToScVal(buf, { type: "bytes" });
}

// Soroban unit-variant enums encode as Symbol
export function toOutcome(outcome: Outcome): StellarSdk.xdr.ScVal {
  return toSymbol(outcome);
}

// Decode helpers
export function fromScVal(val: StellarSdk.xdr.ScVal): unknown {
  return StellarSdk.scValToNative(val);
}

export function fromI128(val: StellarSdk.xdr.ScVal): bigint {
  const native = StellarSdk.scValToNative(val);
  return typeof native === "bigint" ? native : BigInt(String(native));
}

export function fromU64(val: StellarSdk.xdr.ScVal): bigint {
  return fromI128(val);
}

export function fromBool(val: StellarSdk.xdr.ScVal): boolean {
  return Boolean(StellarSdk.scValToNative(val));
}

export function fromString(val: StellarSdk.xdr.ScVal): string {
  return String(StellarSdk.scValToNative(val));
}

export function fromSymbol(val: StellarSdk.xdr.ScVal): string {
  return String(StellarSdk.scValToNative(val));
}

// Format bigint USDC amount (7 decimals) for display
export function formatUsdc(raw: bigint | string): string {
  const n = typeof raw === "string" ? BigInt(raw) : raw;
  const whole = n / 10_000_000n;
  const frac = n % 10_000_000n;
  const fracStr = frac.toString().padStart(7, "0").replace(/0+$/, "");
  return fracStr ? `${whole}.${fracStr}` : `${whole}`;
}

// Parse USDC decimal string to raw bigint (7 decimals)
export function parseUsdc(display: string): bigint {
  const [whole = "0", frac = ""] = display.split(".");
  const fracPadded = frac.padEnd(7, "0").slice(0, 7);
  return BigInt(whole) * 10_000_000n + BigInt(fracPadded);
}

// Implied YES price from AMM reserves (bps, 10000 = 1.0)
export function impliedYesBps(yesReserve: string, noReserve: string): number {
  const yes = BigInt(yesReserve);
  const no = BigInt(noReserve);
  if (yes + no === 0n) return 5000;
  return Number((no * 10000n) / (yes + no));
}

// Slippage / min-out calculation (bps)
export function applySlippage(amountOut: bigint, slippageBps: number): bigint {
  return (amountOut * BigInt(10000 - slippageBps)) / 10000n;
}

import * as StellarSdk from "@stellar/stellar-sdk";
import { getRpc } from "./rpc";
import { networkConfig } from "./config";
import type { DikeErrorCode } from "@/lib/types";

// Mirrors dike_types::DikeError (dike-contracts/crates/dike_types/src/lib.rs).
const DIKE_ERROR_MAP: Record<number, DikeErrorCode> = {
  3: "Unauthorized",
  6: "InvalidStatus",
  15: "InsufficientBalance",
  17: "SlippageExceeded",
  18: "DeadlineExpired",
  30: "UnsupportedCollateral",
  35: "EncumberedPosition",
};

function decodeResultXdr(base64: string) {
  try {
    const result = StellarSdk.xdr.TransactionResult.fromXDR(base64, "base64");
    const outerCode = result.result().switch().name;
    const opResults = result.result().results();
    const firstOpCode = opResults && opResults.length > 0
      ? opResults[0].tr().switch().name
      : null;
    return firstOpCode ? `${outerCode} / ${firstOpCode}` : outerCode;
  } catch {
    return null;
  }
}

export function parseDikeError(err: unknown): string {
  if (err instanceof Error) {
    const msg = err.message;
    // look for Soroban contract error codes
    const match = msg.match(/Error\(Contract, #(\d+)\)/);
    if (match) {
      const code = parseInt(match[1]);
      const named = DIKE_ERROR_MAP[code];
      if (named) return named;
      return `ContractError #${code}`;
    }
    for (const named of Object.values(DIKE_ERROR_MAP)) {
      if (msg.includes(named)) return named;
    }
    const xdrMatch = msg.match(/(AAAA[A-Za-z0-9+/=]+)/);
    if (xdrMatch) {
      const decoded = decodeResultXdr(xdrMatch[1]);
      if (decoded) {
        return msg
          .replace(xdrMatch[1], decoded)
          .replace("Transaction rejected: ", "")
          .replace("Transaction failed on-chain: ", "");
      }
      if (msg.startsWith("Transaction rejected:")) {
        return "Transaction rejected by Stellar RPC";
      }
      if (msg.startsWith("Transaction failed on-chain:")) {
        return "Transaction failed on-chain";
      }
    }
    return msg;
  }
  return String(err);
}

export function verifyNetwork(walletNetwork: string) {
  const expected = networkConfig.networkPassphrase;
  if (walletNetwork !== expected) {
    throw new Error(
      `Wrong network. Wallet is on "${walletNetwork}", expected "${expected}". Switch your wallet to Stellar Testnet.`
    );
  }
}

export function feeFromXdr(xdr: string): string {
  const tx = StellarSdk.TransactionBuilder.fromXDR(
    xdr,
    networkConfig.networkPassphrase
  ) as StellarSdk.Transaction;
  return String(tx.fee);
}

export function formatFeeXlm(stroops: string | number | bigint): string {
  const value = typeof stroops === "bigint" ? stroops : BigInt(stroops);
  const whole = value / 10_000_000n;
  const fraction = (value % 10_000_000n)
    .toString()
    .padStart(7, "0")
    .replace(/0+$/, "");
  return fraction.length > 0 ? `${whole}.${fraction} XLM` : `${whole} XLM`;
}

// Build + simulate a Soroban contract call. Returns assembled XDR ready for signing.
export async function buildAndSimulate(
  sourceAddress: string,
  contractId: string,
  method: string,
  args: StellarSdk.xdr.ScVal[]
): Promise<string> {
  const rpc = getRpc();
  const account = await rpc.getAccount(sourceAddress);
  const contract = new StellarSdk.Contract(contractId);

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: networkConfig.networkPassphrase,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(300)
    .build();

  const simulation = await rpc.simulateTransaction(tx);

  if (StellarSdk.rpc.Api.isSimulationError(simulation)) {
    throw new Error(
      `Simulation failed: ${(simulation as StellarSdk.rpc.Api.SimulateTransactionErrorResponse).error}`
    );
  }

  const assembled = StellarSdk.rpc.assembleTransaction(tx, simulation).build();
  return assembled.toXDR();
}

// Submit a signed XDR and poll for ledger confirmation.
export async function submitAndPoll(
  signedXdr: string
): Promise<{ hash: string; returnValue?: StellarSdk.xdr.ScVal }> {
  const rpc = getRpc();
  const tx = StellarSdk.TransactionBuilder.fromXDR(
    signedXdr,
    networkConfig.networkPassphrase
  ) as StellarSdk.Transaction;

  const sendResp = await rpc.sendTransaction(tx);

  if (sendResp.status === "ERROR") {
    const errResult = sendResp.errorResult;
    throw new Error(`Transaction rejected: ${errResult?.toXDR("base64")}`);
  }

  const hash = sendResp.hash;

  let getResp = await rpc.getTransaction(hash);
  let attempts = 0;
  while (
    getResp.status === StellarSdk.rpc.Api.GetTransactionStatus.NOT_FOUND &&
    attempts < 60
  ) {
    await new Promise((r) => setTimeout(r, 2000));
    getResp = await rpc.getTransaction(hash);
    attempts++;
  }

  if (getResp.status === StellarSdk.rpc.Api.GetTransactionStatus.SUCCESS) {
    const successResp =
      getResp as StellarSdk.rpc.Api.GetSuccessfulTransactionResponse;
    return { hash, returnValue: successResp.returnValue };
  }

  if (getResp.status === StellarSdk.rpc.Api.GetTransactionStatus.FAILED) {
    const failResp =
      getResp as StellarSdk.rpc.Api.GetFailedTransactionResponse;
    throw new Error(
      `Transaction failed on-chain: ${failResp.resultXdr?.toXDR("base64") ?? "unknown"}`
    );
  }

  throw new Error("Transaction not confirmed after polling timeout.");
}

// Simulate a read-only contract call and return the ScVal result.
export async function simulateRead(
  sourceAddress: string,
  contractId: string,
  method: string,
  args: StellarSdk.xdr.ScVal[]
): Promise<StellarSdk.xdr.ScVal> {
  const rpc = getRpc();
  const account = await rpc.getAccount(sourceAddress);
  const contract = new StellarSdk.Contract(contractId);

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: networkConfig.networkPassphrase,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(300)
    .build();

  const simulation = await rpc.simulateTransaction(tx);

  if (StellarSdk.rpc.Api.isSimulationError(simulation)) {
    throw new Error(
      `Read simulation failed: ${(simulation as StellarSdk.rpc.Api.SimulateTransactionErrorResponse).error}`
    );
  }

  const success =
    simulation as StellarSdk.rpc.Api.SimulateTransactionSuccessResponse;
  if (!success.result?.retval) {
    throw new Error("No return value from simulation");
  }
  return success.result.retval;
}

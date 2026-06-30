import * as StellarSdk from "@stellar/stellar-sdk";
import { buildAndSimulate, simulateRead } from "@/lib/stellar/transaction";
import {
  toAddress,
  toI128,
  toU64,
  toU32,
  toSymbol,
  toString,
  toBytes,
  toOutcome,
  fromI128,
  fromBool,
  fromScVal,
} from "@/lib/stellar/scval";
import { CONTRACT_IDS, COLLATERAL_CONTRACT } from "./manifest";
import type {
  Outcome,
  MarketData,
  PoolData,
  TradeQuote,
  ResolutionRequest,
  CouncilCase,
  FeeConfig,
} from "@/lib/types";

// ─── Defaults ───────────────────────────────────────────────────────────────

export const DEFAULT_FEE_CONFIG: FeeConfig = {
  tradingFeeBps: 200,
  lpFeeShareBps: 7000,
  treasuryFeeShareBps: 2000,
  codFeeShareBps: 1000,
  proposalReward: "0",
  disputeReward: "0",
  councilReward: "0",
  creationFee: "0",
};

// ─── Helpers ────────────────────────────────────────────────────────────────

function amt(n: string): StellarSdk.xdr.ScVal {
  return toI128(BigInt(n));
}

function deadline(seconds = 300): StellarSdk.xdr.ScVal {
  const ts = BigInt(Math.floor(Date.now() / 1000) + seconds);
  return toU64(ts);
}

function id(n: string | number): StellarSdk.xdr.ScVal {
  return toU64(BigInt(n));
}

// Build a Soroban struct ScMap with sorted symbol keys.
// Soroban #[contracttype] structs encode as ScMap<ScvSymbol, ScVal> with lexicographic key order.
function scvStruct(fields: [string, StellarSdk.xdr.ScVal][]): StellarSdk.xdr.ScVal {
  return StellarSdk.xdr.ScVal.scvMap(
    [...fields]
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(
        ([k, v]) =>
          new StellarSdk.xdr.ScMapEntry({
            key: StellarSdk.xdr.ScVal.scvSymbol(k),
            val: v,
          })
      )
  );
}

function feeConfigScVal(fee: FeeConfig): StellarSdk.xdr.ScVal {
  return scvStruct([
    ["cod_fee_share_bps", toU32(fee.codFeeShareBps)],
    ["council_reward", toI128(BigInt(fee.councilReward))],
    ["creation_fee", toI128(BigInt(fee.creationFee))],
    ["dispute_reward", toI128(BigInt(fee.disputeReward))],
    ["lp_fee_share_bps", toU32(fee.lpFeeShareBps)],
    ["proposal_reward", toI128(BigInt(fee.proposalReward))],
    ["trading_fee_bps", toU32(fee.tradingFeeBps)],
    ["treasury_fee_share_bps", toU32(fee.treasuryFeeShareBps)],
  ]);
}

// ─── AMM ────────────────────────────────────────────────────────────────────

export async function ammQuoteBuyYes(
  source: string,
  poolId: string,
  amountIn: string
): Promise<TradeQuote> {
  const val = await simulateRead(source, CONTRACT_IDS.amm(), "quote_buy_yes", [
    id(poolId),
    amt(amountIn),
  ]);
  const native = fromScVal(val) as { amount_out: string; fee_bps: number };
  return {
    amountIn,
    amountOut: String(native.amount_out),
    priceImpactBps: 0,
    feeBps: native.fee_bps ?? 0,
  };
}

export async function ammQuoteBuyNo(
  source: string,
  poolId: string,
  amountIn: string
): Promise<TradeQuote> {
  const val = await simulateRead(source, CONTRACT_IDS.amm(), "quote_buy_no", [
    id(poolId),
    amt(amountIn),
  ]);
  const native = fromScVal(val) as { amount_out: string; fee_bps: number };
  return {
    amountIn,
    amountOut: String(native.amount_out),
    priceImpactBps: 0,
    feeBps: native.fee_bps ?? 0,
  };
}

export async function buildAmmBuyYes(
  trader: string,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "buy_yes", [
    toAddress(trader),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmBuyNo(
  trader: string,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "buy_no", [
    toAddress(trader),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmBuyChildYes(
  trader: string,
  parentMarketId: string,
  parentOutcome: Outcome,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "buy_child_yes", [
    toAddress(trader),
    id(parentMarketId),
    toOutcome(parentOutcome),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmBuyChildNo(
  trader: string,
  parentMarketId: string,
  parentOutcome: Outcome,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "buy_child_no", [
    toAddress(trader),
    id(parentMarketId),
    toOutcome(parentOutcome),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmSellYes(
  trader: string,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "sell_yes", [
    toAddress(trader),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmSellNo(
  trader: string,
  poolId: string,
  amountIn: string,
  minOut: string,
  deadlineSecs = 300
): Promise<string> {
  return buildAndSimulate(trader, CONTRACT_IDS.amm(), "sell_no", [
    toAddress(trader),
    id(poolId),
    amt(amountIn),
    amt(minOut),
    deadline(deadlineSecs),
  ]);
}

export async function buildAmmAddLiquidity(
  lp: string,
  poolId: string,
  amount: string
): Promise<string> {
  return buildAndSimulate(lp, CONTRACT_IDS.amm(), "add_liquidity", [
    toAddress(lp),
    id(poolId),
    amt(amount),
  ]);
}

export async function buildAmmRemoveLiquidity(
  lp: string,
  poolId: string,
  shares: string
): Promise<string> {
  return buildAndSimulate(lp, CONTRACT_IDS.amm(), "remove_liquidity", [
    toAddress(lp),
    id(poolId),
    amt(shares),
  ]);
}

export async function ammGetPool(
  source: string,
  poolId: string
): Promise<PoolData> {
  const val = await simulateRead(source, CONTRACT_IDS.amm(), "pool", [
    id(poolId),
  ]);
  const native = fromScVal(val) as Record<string, unknown>;
  return {
    poolId,
    marketId: String(native.market_id ?? ""),
    yesReserve: String(native.yes_reserve ?? "0"),
    noReserve: String(native.no_reserve ?? "0"),
    lpSupply: String(native.lp_supply ?? "0"),
    feeBps: Number(native.fee_bps ?? 0),
  };
}

// Contract: lp_balance(pool_id: u64, owner: Address) -> i128
export async function ammGetLpBalance(
  source: string,
  lp: string,
  poolId: string
): Promise<string> {
  const val = await simulateRead(source, CONTRACT_IDS.amm(), "lp_balance", [
    id(poolId),
    toAddress(lp),
  ]);
  return String(fromI128(val));
}

// ─── CollateralVault ─────────────────────────────────────────────────────────

// Contract: redeem_resolved(token, user, market_id: u64, redeemed_outcome, amount)
export async function buildRedeemResolved(
  user: string,
  marketId: string,
  redeemedOutcome: Outcome,
  amount: string
): Promise<string> {
  return buildAndSimulate(
    user,
    CONTRACT_IDS.collateralVault(),
    "redeem_resolved",
    [
      toAddress(COLLATERAL_CONTRACT),
      toAddress(user),
      id(marketId),
      toOutcome(redeemedOutcome),
      amt(amount),
    ]
  );
}

// Contract: redeem_cancelled(token, user, market_id: u64, redeemed_outcome, amount)
export async function buildRedeemCancelled(
  user: string,
  marketId: string,
  redeemedOutcome: Outcome,
  amount: string
): Promise<string> {
  return buildAndSimulate(
    user,
    CONTRACT_IDS.collateralVault(),
    "redeem_cancelled",
    [
      toAddress(COLLATERAL_CONTRACT),
      toAddress(user),
      id(marketId),
      toOutcome(redeemedOutcome),
      amt(amount),
    ]
  );
}

// Contract: user_deposit(market_id: u64, user: Address) -> i128
export async function vaultGetUserDeposit(
  source: string,
  user: string,
  marketId: string
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "user_deposit",
    [id(marketId), toAddress(user)]
  );
  return String(fromI128(val));
}

// Contract: root_stake(market_id: u64, user: Address, outcome: Outcome) -> i128
export async function vaultGetRootStake(
  source: string,
  user: string,
  marketId: string,
  outcome: Outcome
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "root_stake",
    [id(marketId), toAddress(user), toOutcome(outcome)]
  );
  return String(fromI128(val));
}

// Contract: child_avail_for_outcome(parent_market_id: u64, user: Address, outcome: Outcome) -> i128
export async function vaultGetChildAvail(
  source: string,
  user: string,
  parentMarketId: string,
  outcome: Outcome
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "child_avail_for_outcome",
    [id(parentMarketId), toAddress(user), toOutcome(outcome)]
  );
  return String(fromI128(val));
}

// Contract: child_debt(child_market_id: u64, user: Address) -> i128
export async function vaultGetChildDebt(
  source: string,
  user: string,
  childMarketId: string
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "child_debt",
    [id(childMarketId), toAddress(user)]
  );
  return String(fromI128(val));
}

// Contract: parent_debt(parent_market_id: u64, user: Address, outcome: Outcome) -> i128
export async function vaultGetParentDebt(
  source: string,
  user: string,
  parentMarketId: string,
  outcome: Outcome
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "parent_debt",
    [id(parentMarketId), toAddress(user), toOutcome(outcome)]
  );
  return String(fromI128(val));
}

// ─── MarketRegistry ──────────────────────────────────────────────────────────

export async function registryGetMarket(
  source: string,
  marketId: string
): Promise<MarketData> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.marketRegistry(),
    "get_market",
    [id(marketId)]
  );
  return fromScVal(val) as MarketData;
}

export async function registryIsTradeable(
  source: string,
  marketId: string
): Promise<boolean> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.marketRegistry(),
    "is_tradeable",
    [id(marketId)]
  );
  return fromBool(val);
}

export async function registryGetFinalOutcome(
  source: string,
  marketId: string
): Promise<Outcome | null> {
  try {
    const val = await simulateRead(
      source,
      CONTRACT_IDS.marketRegistry(),
      "get_final_outcome",
      [id(marketId)]
    );
    return fromScVal(val) as Outcome;
  } catch {
    return null;
  }
}

// ─── ConditionalTokens ───────────────────────────────────────────────────────

// Contract: balance(market_id: u64, owner: Address, outcome: Outcome) -> i128
export async function ctBalance(
  source: string,
  account: string,
  marketId: string,
  outcome: Outcome
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.conditionalTokens(),
    "balance",
    [id(marketId), toAddress(account), toOutcome(outcome)]
  );
  return String(fromI128(val));
}

// ─── CODOracle ───────────────────────────────────────────────────────────────

// Contract: request_resolution(market_id: u64, question_hash: BytesN<32>, rules_uri, expiry, bond_amount, dispute_window: u64)
export async function buildRequestResolution(
  requester: string,
  marketId: string,
  questionHash: string,
  rulesUri: string,
  expiry: number,
  bondAmount: string,
  disputeWindow: number
): Promise<string> {
  return buildAndSimulate(
    requester,
    CONTRACT_IDS.codOracle(),
    "request_resolution",
    [
      id(marketId),
      toBytes(questionHash),
      toString(rulesUri),
      toU64(BigInt(expiry)),
      amt(bondAmount),
      toU64(BigInt(disputeWindow)),
    ]
  );
}

// Contract: propose_outcome(proposer, request_id: u64, outcome, evidence_uri)
export async function buildProposeOutcome(
  proposer: string,
  requestId: string,
  outcome: Outcome,
  evidenceUri: string
): Promise<string> {
  return buildAndSimulate(
    proposer,
    CONTRACT_IDS.codOracle(),
    "propose_outcome",
    [
      toAddress(proposer),
      id(requestId),
      toOutcome(outcome),
      toString(evidenceUri),
    ]
  );
}

// Contract: dispute_outcome(disputer, request_id: u64, counter_outcome, evidence_uri)
export async function buildDisputeOutcome(
  disputer: string,
  requestId: string,
  counterOutcome: Outcome,
  evidenceUri: string
): Promise<string> {
  return buildAndSimulate(
    disputer,
    CONTRACT_IDS.codOracle(),
    "dispute_outcome",
    [
      toAddress(disputer),
      id(requestId),
      toOutcome(counterOutcome),
      toString(evidenceUri),
    ]
  );
}

// Contract: finalize_undisputed(request_id: u64)
export async function buildFinalizeUndisputed(
  caller: string,
  requestId: string
): Promise<string> {
  return buildAndSimulate(
    caller,
    CONTRACT_IDS.codOracle(),
    "finalize_undisputed",
    [id(requestId)]
  );
}

// Contract: escalate_to_council(request_id: u64)
export async function buildEscalateToCouncil(
  caller: string,
  requestId: string
): Promise<string> {
  return buildAndSimulate(
    caller,
    CONTRACT_IDS.codOracle(),
    "escalate_to_council",
    [id(requestId)]
  );
}

// Contract: request(request_id: u64)
export async function oracleGetRequest(
  source: string,
  requestId: string
): Promise<ResolutionRequest> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.codOracle(),
    "request",
    [id(requestId)]
  );
  return fromScVal(val) as ResolutionRequest;
}

// ─── CouncilOfDike ───────────────────────────────────────────────────────────

// Contract: vote_commitment(case_id: u64, voter, outcome, salt: BytesN<32>)
export async function councilCalcCommitment(
  source: string,
  caseId: string,
  voter: string,
  outcome: Outcome,
  salt: string
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.councilOfDike(),
    "vote_commitment",
    [
      id(caseId),
      toAddress(voter),
      toOutcome(outcome),
      toBytes(salt),
    ]
  );
  const native = fromScVal(val);
  return String(native);
}

// Contract: commit_vote(voter, case_id: u64, commitment: BytesN<32>)
export async function buildCommitVote(
  voter: string,
  caseId: string,
  commitment: string
): Promise<string> {
  return buildAndSimulate(
    voter,
    CONTRACT_IDS.councilOfDike(),
    "commit_vote",
    [toAddress(voter), id(caseId), toBytes(commitment)]
  );
}

// Contract: reveal_vote(voter, case_id: u64, outcome, salt: BytesN<32>)
export async function buildRevealVote(
  voter: string,
  caseId: string,
  outcome: Outcome,
  salt: string
): Promise<string> {
  return buildAndSimulate(
    voter,
    CONTRACT_IDS.councilOfDike(),
    "reveal_vote",
    [toAddress(voter), id(caseId), toOutcome(outcome), toBytes(salt)]
  );
}

// Contract: finalize_and_report_case(case_id: u64)
export async function buildFinalizeCase(
  caller: string,
  caseId: string
): Promise<string> {
  return buildAndSimulate(
    caller,
    CONTRACT_IDS.councilOfDike(),
    "finalize_and_report_case",
    [id(caseId)]
  );
}

// Contract: claim_reward(voter, case_id: u64)
export async function buildClaimReward(
  voter: string,
  caseId: string
): Promise<string> {
  return buildAndSimulate(
    voter,
    CONTRACT_IDS.councilOfDike(),
    "claim_reward",
    [toAddress(voter), id(caseId)]
  );
}

// Contract: case(case_id: u64)
export async function councilGetCase(
  source: string,
  caseId: string
): Promise<CouncilCase> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.councilOfDike(),
    "case",
    [id(caseId)]
  );
  return fromScVal(val) as CouncilCase;
}

// ─── MarketFactory ───────────────────────────────────────────────────────────

export interface CreateMarketParams {
  question: string;
  questionHash: string;
  rulesUri: string;
  rulesHash: string;
  category: string;
  expiry: number;
  collateral: string;
  bondAmount: string;
  disputeWindow: number;
  feeConfig?: FeeConfig;
}

// Contract: create_market(config: MarketConfig, initial_liquidity: i128, opening_price_bps: u32)
// MarketConfig is a Soroban #[contracttype] struct → scvMap with symbol keys, alphabetically sorted.
export async function buildCreateMarket(
  creator: string,
  params: CreateMarketParams,
  initialLiquidity: string,
  openingPriceBps = 5000
): Promise<string> {
  const fee = params.feeConfig ?? DEFAULT_FEE_CONFIG;
  const configVal = scvStruct([
    ["bond_amount", toI128(BigInt(params.bondAmount))],
    ["category", toString(params.category)],
    ["collateral", toAddress(params.collateral)],
    ["creator", toAddress(creator)],
    ["dispute_window", toU64(BigInt(params.disputeWindow))],
    ["expiry", toU64(BigInt(params.expiry))],
    ["fee_config", feeConfigScVal(fee)],
    ["question", toString(params.question)],
    ["question_hash", toBytes(params.questionHash)],
    ["rules_hash", toBytes(params.rulesHash)],
    ["rules_uri", toString(params.rulesUri)],
  ]);

  return buildAndSimulate(
    creator,
    CONTRACT_IDS.marketFactory(),
    "create_market",
    [configVal, amt(initialLiquidity), toU32(openingPriceBps)]
  );
}

import * as StellarSdk from "@stellar/stellar-sdk";
import { buildAndSimulate, simulateRead } from "@/lib/stellar/transaction";
import {
  toAddress,
  toI128,
  toU64,
  toU32,
  toString,
  toBytes,
  toOutcome,
  fromI128,
  fromBool,
  fromScVal,
  impliedYesBps,
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

function feeBpsFromQuote(fee: bigint, grossAmount: bigint): number {
  if (grossAmount <= 0n) return 0;
  return Number((fee * 10_000n) / grossAmount);
}

function priceImpactFromQuote(
  averagePriceBps: number,
  yesReserve: string,
  noReserve: string,
  token: "yes" | "no",
): number {
  const yesSpot = impliedYesBps(yesReserve, noReserve);
  const spot = token === "yes" ? yesSpot : 10_000 - yesSpot;
  return Math.abs(averagePriceBps - spot);
}

async function readQuote(
  source: string,
  poolId: string,
  method: "quote_buy_yes" | "quote_buy_no" | "quote_sell_yes" | "quote_sell_no",
  amountIn: string,
  token: "yes" | "no",
): Promise<TradeQuote> {
  const [val, pool] = await Promise.all([
    simulateRead(source, CONTRACT_IDS.amm(), method, [id(poolId), amt(amountIn)]),
    ammGetPool(source, poolId),
  ]);
  const native = fromScVal(val) as {
    amount_out: string;
    fee: string;
    net_in: string;
    average_price_bps: number;
  };
  const fee = BigInt(String(native.fee ?? "0"));
  const netIn = BigInt(String(native.net_in ?? "0"));
  const grossAmount = fee + netIn;
  const averagePriceBps = Number(native.average_price_bps ?? 0);
  return {
    amountIn,
    amountOut: String(native.amount_out),
    priceImpactBps: priceImpactFromQuote(
      averagePriceBps,
      pool.yesReserve,
      pool.noReserve,
      token,
    ),
    feeBps: feeBpsFromQuote(fee, grossAmount),
  };
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
    ["lp_fee_share_bps", toU32(fee.lpFeeShareBps)],
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
  return readQuote(source, poolId, "quote_buy_yes", amountIn, "yes");
}

export async function ammQuoteBuyNo(
  source: string,
  poolId: string,
  amountIn: string
): Promise<TradeQuote> {
  return readQuote(source, poolId, "quote_buy_no", amountIn, "no");
}

export async function ammQuoteSellYes(
  source: string,
  poolId: string,
  amountIn: string
): Promise<TradeQuote> {
  return readQuote(source, poolId, "quote_sell_yes", amountIn, "yes");
}

export async function ammQuoteSellNo(
  source: string,
  poolId: string,
  amountIn: string
): Promise<TradeQuote> {
  return readQuote(source, poolId, "quote_sell_no", amountIn, "no");
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

// Contract: claim_lp_fees(lp: Address, pool_id: u64) -> i128
export async function buildAmmClaimLpFees(
  lp: string,
  poolId: string
): Promise<string> {
  return buildAndSimulate(lp, CONTRACT_IDS.amm(), "claim_lp_fees", [
    toAddress(lp),
    id(poolId),
  ]);
}

// Contract: claimable_lp_fees(pool_id: u64, owner: Address) -> i128
export async function ammGetClaimableLpFees(
  source: string,
  lp: string,
  poolId: string
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.amm(),
    "claimable_lp_fees",
    [id(poolId), toAddress(lp)]
  );
  return String(fromI128(val));
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
    lpSupply: String(native.total_lp_shares ?? "0"),
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

// Contract: accounting(market_id: u64) -> VaultAccounting
// Low-frequency admin-only read, fine to stay on RPC (only touched from the
// sweep-protocol-fees panel, not on any page-mount burst).
export async function vaultGetAccounting(
  source: string,
  marketId: string
): Promise<{ protocolFees: string; codFees: string }> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "accounting",
    [id(marketId)]
  );
  const native = fromScVal(val) as { protocol_fees?: string; cod_fees?: string };
  return {
    protocolFees: String(native.protocol_fees ?? "0"),
    codFees: String(native.cod_fees ?? "0"),
  };
}

// Contract: sweep_protocol_fees(token, market_id: u64) -> i128
// Gated on-chain by require_role("gov") — succeeds only when signed by
// whatever address collateral_vault has registered under that role
// (governance_authority in this deployment).
export async function buildSweepProtocolFees(
  source: string,
  marketId: string
): Promise<string> {
  return buildAndSimulate(
    source,
    CONTRACT_IDS.collateralVault(),
    "sweep_protocol_fees",
    [toAddress(COLLATERAL_CONTRACT), id(marketId)]
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

// Contract: redeemed(market_id: u64, user: Address, outcome: Outcome) -> i128
export async function vaultGetRedeemed(
  source: string,
  user: string,
  marketId: string,
  outcome: Outcome
): Promise<string> {
  const val = await simulateRead(
    source,
    CONTRACT_IDS.collateralVault(),
    "redeemed",
    [id(marketId), toAddress(user), toOutcome(outcome)]
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

// Contract: close_trading(market_id: u64) — permissionless, callable by anyone
// once env.ledger().timestamp() >= market.expiry. Errors with NotExpired otherwise.
export async function buildCloseTrading(
  source: string,
  marketId: string
): Promise<string> {
  return buildAndSimulate(source, CONTRACT_IDS.marketRegistry(), "close_trading", [
    id(marketId),
  ]);
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

// Contract: balance(owner: Address, market_id: u64, outcome: Outcome) -> i128
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
    [toAddress(account), id(marketId), toOutcome(outcome)]
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

// ─── DikeTimelock ────────────────────────────────────────────────────────────

// Contract: execute(action_id: u64) -> TimelockAction
export async function buildTimelockExecute(
  caller: string,
  actionId: string
): Promise<string> {
  return buildAndSimulate(
    caller,
    CONTRACT_IDS.dikeTimelock(),
    "execute",
    [id(actionId)]
  );
}

// ─── FeeManager ──────────────────────────────────────────────────────────────

// Contract: config() -> FeeConfig
export async function feeManagerGetConfig(source: string): Promise<FeeConfig> {
  const val = await simulateRead(source, CONTRACT_IDS.feeManager(), "config", []);
  const native = fromScVal(val) as Record<string, unknown>;
  return {
    tradingFeeBps: Number(native.trading_fee_bps ?? 0),
    lpFeeShareBps: Number(native.lp_fee_share_bps ?? 0),
    treasuryFeeShareBps: Number(native.treasury_fee_share_bps ?? 0),
    codFeeShareBps: Number(native.cod_fee_share_bps ?? 0),
    councilReward: String(native.council_reward ?? "0"),
    creationFee: String(native.creation_fee ?? "0"),
  };
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

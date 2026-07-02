import type {
  AdminState,
  CouncilCase,
  MarketData,
  MarketStatus,
  Outcome,
  ResolutionRequest,
  TimelockAction,
  UserPosition,
  WalletPermissions,
} from "@/lib/types";

type RawRecord = Record<string, unknown>;

function toStringValue(value: unknown, fallback = "") {
  return typeof value === "string" ? value : value == null ? fallback : String(value);
}

function toNumberValue(value: unknown, fallback = 0) {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.length) return Number(value);
  return fallback;
}

function toNullableString(value: unknown) {
  if (value == null) return null;
  const stringValue = toStringValue(value);
  return stringValue.length ? stringValue : null;
}

function toBooleanValue(value: unknown, fallback = false) {
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") {
    if (value === "true" || value === "1" || value.toLowerCase() === "yes") return true;
    if (value === "false" || value === "0" || value.toLowerCase() === "no") return false;
  }
  return fallback;
}

function toOutcome(value: unknown): Outcome | null {
  return value === "Yes" || value === "No" || value === "Invalid" ? value : null;
}

function toMarketStatus(value: unknown): MarketStatus {
  const status = toStringValue(value, "Created");
  return (
    [
      "Created",
      "Live",
      "Paused",
      "TradingClosed",
      "ResolutionRequested",
      "Proposed",
      "Disputed",
      "CouncilVoting",
      "Resolved",
      "Cancelled",
    ] as MarketStatus[]
  ).includes(status as MarketStatus)
    ? (status as MarketStatus)
    : "Created";
}

export function normalizeMarketData(row: RawRecord): MarketData {
  const hasFinalOutcome = Boolean(row.has_final_outcome);
  const hasRequest = Boolean(row.has_request);
  const poolId = toStringValue(row.pool_id ?? "");

  return {
    marketId: toStringValue(row.market_id),
    config: {
      question: toStringValue(row.question),
      questionHash: toStringValue(row.question_hash),
      rulesUri: toStringValue(row.rules_uri),
      rulesHash: toStringValue(row.rules_hash),
      category: toStringValue(row.category, "General"),
      expiry: toNumberValue(row.expiry),
      collateral: toStringValue(row.collateral),
      bondAmount: toStringValue(row.bond_amount, "0"),
      disputeWindow: toNumberValue(row.dispute_window),
      creator: toStringValue(row.creator),
    },
    status: toMarketStatus(row.status),
    finalOutcome: hasFinalOutcome ? toOutcome(row.final_outcome) : null,
    requestId: hasRequest ? toStringValue(row.request_id) : null,
    poolId: poolId && poolId !== "0" ? poolId : null,
    yesReserve: toStringValue(row.yes_reserve, "0"),
    noReserve: toStringValue(row.no_reserve, "0"),
    createdAt: toNumberValue(row.created_at_unix),
  };
}

export function normalizeResolutionRequest(row: RawRecord | null): ResolutionRequest | null {
  if (!row) return null;

  return {
    requestId: toStringValue(row.request_id ?? row.id),
    marketId: toStringValue(row.market_id),
    questionHash: toStringValue(row.question_hash),
    rulesUri: toStringValue(row.rules_uri),
    expiry: toNumberValue(row.expiry),
    bondAmount: toStringValue(row.bond_amount, "0"),
    disputeWindow: toNumberValue(row.dispute_window),
    status: toStringValue(row.status, "None") as ResolutionRequest["status"],
    proposedOutcome: Boolean(row.has_proposal) ? toOutcome(row.proposed_outcome) : null,
    proposedAt: row.proposed_at == null ? null : toNumberValue(row.proposed_at),
    proposer: Boolean(row.has_proposal) ? toNullableString(row.proposer) : null,
    disputer: Boolean(row.has_dispute) ? toNullableString(row.disputer) : null,
    disputedOutcome: Boolean(row.has_dispute) ? toOutcome(row.disputed_outcome) : null,
    counterEvidenceUri: Boolean(row.has_dispute) ? toNullableString(row.dispute_evidence_uri) : null,
    evidenceUri: Boolean(row.has_proposal) ? toNullableString(row.proposal_evidence_uri) : null,
  };
}

export function normalizeCouncilCase(row: RawRecord): CouncilCase {
  const hasFinalOutcome = Boolean(row.has_final_outcome);

  return {
    caseId: toStringValue(row.case_id ?? row.id),
    requestId: toStringValue(row.request_id),
    status: toStringValue(row.status, "Opened") as CouncilCase["status"],
    proposedOutcome: (toOutcome(row.proposer_outcome) ?? "Invalid") as Outcome,
    disputedOutcome: (toOutcome(row.disputer_outcome) ?? "Invalid") as Outcome,
    commitEnd: toNumberValue(row.commit_end),
    revealEnd: toNumberValue(row.reveal_end),
    finalOutcome: hasFinalOutcome ? toOutcome(row.final_outcome) : null,
    evidenceUri:
      toStringValue(row.proposer_evidence_uri) || toStringValue(row.disputer_evidence_uri),
  };
}

export function normalizePortfolio(
  portfolio: {
    positions: RawRecord[];
    lpPositions: RawRecord[];
    vaultState: RawRecord[];
  },
  markets: MarketData[],
): UserPosition[] {
  const marketMap = new Map(markets.map((market) => [market.marketId, market]));
  const lpByPool = new Map(
    portfolio.lpPositions.map((position) => [toStringValue(position.pool_id), position]),
  );

  return portfolio.vaultState.map((vault) => {
    const marketId = toStringValue(vault.market_id);
    const market = marketMap.get(marketId);

    const positionMap = new Map(
      portfolio.positions
        .filter((position) => toStringValue(position.market_id) === marketId)
        .map((position) => [toStringValue(position.outcome), toStringValue(position.balance, "0")]),
    );

    const lpShares =
      market?.poolId && lpByPool.has(market.poolId)
        ? toStringValue(lpByPool.get(market.poolId)?.shares, "0")
        : "0";

    return {
      marketId,
      poolId: market?.poolId ?? null,
      question: market?.config.question ?? `Market #${marketId}`,
      yesBalance: positionMap.get("Yes") ?? "0",
      noBalance: positionMap.get("No") ?? "0",
      lpShares,
      deposit: toStringValue(vault.user_deposit, "0"),
      rootStake: toStringValue(vault.root_stake_yes, "0"),
      childCredit: toStringValue(vault.child_used_total, "0"),
      childDebt: toStringValue(vault.child_debt, "0"),
      parentDebt: toStringValue(vault.parent_debt_yes, "0"),
      redeemedAmount: toStringValue(vault.redeemed_yes, "0"),
      marketStatus: market?.status ?? "Created",
      finalOutcome: market?.finalOutcome ?? null,
    };
  });
}

export function normalizeTimelockAction(row: RawRecord): TimelockAction {
  const actionId = toStringValue(row.action_id ?? row.actionId);
  const executed = toBooleanValue(row.executed);
  const cancelled = toBooleanValue(row.cancelled);
  const eta = toNumberValue(row.execute_after ?? row.eta);
  const target = toNullableString(row.target);
  const payloadHash = toNullableString(row.payload_hash);
  const expiresAt = toNullableString(row.expires_at);

  return {
    actionId,
    kind: toStringValue(row.kind, "Upgrade") as TimelockAction["kind"],
    data: [target, payloadHash, expiresAt ? `expires:${expiresAt}` : null]
      .filter((value): value is string => Boolean(value))
      .join(" | "),
    payload: row.payload_json ?? null,
    eta,
    queued: !executed && !cancelled && eta > 0,
    executed,
    cancelled,
  };
}

export function normalizeAdminState(response: {
  config: RawRecord | null;
  lists: RawRecord[];
  modules: RawRecord[];
}): AdminState {
  const feeConfig = (response.config?.fee_config_json as RawRecord | undefined) ?? {};
  const approvedLists = response.lists.filter((entry) => toBooleanValue(entry.approved, true));

  return {
    treasury: toStringValue(response.config?.treasury, "—"),
    supportedCollaterals: approvedLists
      .filter((entry) => toStringValue(entry.kind) === "collateral")
      .map((entry) => toStringValue(entry.address))
      .filter(Boolean),
    feeConfig: {
      tradingFeeBps: toNumberValue(feeConfig.trading_fee_bps),
      lpFeeShareBps: toNumberValue(feeConfig.lp_fee_share_bps),
      treasuryFeeShareBps: toNumberValue(feeConfig.treasury_fee_share_bps),
      codFeeShareBps: toNumberValue(feeConfig.cod_fee_share_bps),
      councilReward: toStringValue(feeConfig.council_reward, "0"),
      creationFee: toStringValue(feeConfig.creation_fee, "0"),
    },
    moduleAddresses: Object.fromEntries(
      response.modules.map((entry) => [
        toStringValue(entry.role),
        toStringValue(entry.module_address),
      ]),
    ),
    approvedCreators: approvedLists
      .filter((entry) => toStringValue(entry.kind) === "creator")
      .map((entry) => toStringValue(entry.address))
      .filter(Boolean),
    councilMembers: approvedLists
      .filter((entry) => toStringValue(entry.kind) === "member")
      .map((entry) => toStringValue(entry.address))
      .filter(Boolean),
  };
}

export function normalizeWalletPermissions(row: RawRecord): WalletPermissions {
  const isApprovedCreator = toBooleanValue(row.isApprovedCreator);
  const isCouncilMember = toBooleanValue(row.isCouncilMember);
  const isAdmin = toBooleanValue(row.isAdmin);

  return {
    address: toStringValue(row.address).toUpperCase(),
    canCreate: toBooleanValue(row.canCreate, isApprovedCreator),
    canResolve: toBooleanValue(row.canResolve),
    canCouncil: toBooleanValue(row.canCouncil, isCouncilMember),
    canAdmin: toBooleanValue(row.canAdmin, isAdmin),
    isApprovedCreator,
    isCouncilMember,
    isAdmin,
  };
}

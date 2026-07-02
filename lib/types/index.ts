export const MARKET_CATEGORIES = ["Politics", "Sports", "Crypto", "Business", "General"] as const;
export type MarketCategory = (typeof MARKET_CATEGORIES)[number];

export type MarketStatus =
  | "Created"
  | "Live"
  | "Paused"
  | "TradingClosed"
  | "ResolutionRequested"
  | "Proposed"
  | "Disputed"
  | "CouncilVoting"
  | "Resolved"
  | "Cancelled";

export type Outcome = "Yes" | "No" | "Invalid";

export type OracleStatus =
  | "None"
  | "Requested"
  | "Proposed"
  | "Disputed"
  | "Escalated"
  | "Finalized";

export type CouncilCaseStatus =
  | "Opened"
  | "CommitPhase"
  | "RevealPhase"
  | "ReadyToFinalize"
  | "Finalized"
  | "Cancelled";

export type TimelockActionKind =
  | "FeeConfig"
  | "Treasury"
  | "SupportedCollateral"
  | "Creator"
  | "CouncilMember"
  | "ModuleAddress"
  | "Pause"
  | "Upgrade";

export interface FeeConfig {
  tradingFeeBps: number;
  lpFeeShareBps: number;
  treasuryFeeShareBps: number;
  codFeeShareBps: number;
  councilReward: string;
  creationFee: string;
}

export interface MarketConfig {
  question: string;
  questionHash: string;
  rulesUri: string;
  rulesHash: string;
  category: string;
  expiry: number;
  collateral: string;
  bondAmount: string;
  disputeWindow: number;
  creator: string;
}

export interface MarketData {
  marketId: string;
  config: MarketConfig;
  status: MarketStatus;
  finalOutcome: Outcome | null;
  requestId: string | null;
  poolId: string | null;
  yesReserve: string;
  noReserve: string;
  createdAt: number;
}

export interface VaultAccounting {
  totalDeposits: string;
  totalRootStake: string;
  totalChildCredit: string;
  totalChildDebt: string;
  totalParentDebt: string;
}

export interface PoolData {
  poolId: string;
  marketId: string;
  yesReserve: string;
  noReserve: string;
  lpSupply: string;
  feeBps: number;
}

export interface TradeQuote {
  amountIn: string;
  amountOut: string;
  priceImpactBps: number;
  feeBps: number;
}

export interface ResolutionRequest {
  requestId: string;
  marketId: string;
  questionHash: string;
  rulesUri: string;
  expiry: number;
  bondAmount: string;
  disputeWindow: number;
  status: OracleStatus;
  proposedOutcome: Outcome | null;
  proposedAt: number | null;
  proposer: string | null;
  disputer: string | null;
  disputedOutcome: Outcome | null;
  counterEvidenceUri: string | null;
  evidenceUri: string | null;
}

export interface CouncilCase {
  caseId: string;
  requestId: string;
  status: CouncilCaseStatus;
  proposedOutcome: Outcome;
  disputedOutcome: Outcome;
  commitEnd: number;
  revealEnd: number;
  finalOutcome: Outcome | null;
  evidenceUri: string;
}

export interface TimelockAction {
  actionId: string;
  kind: TimelockActionKind;
  data: string;
  payload: unknown;
  eta: number;
  queued: boolean;
  executed: boolean;
  cancelled: boolean;
}

export interface UserPosition {
  marketId: string;
  poolId: string | null;
  question: string;
  yesBalance: string;
  noBalance: string;
  lpShares: string;
  deposit: string;
  rootStake: string;
  childCredit: string;
  childDebt: string;
  parentDebt: string;
  redeemedAmount: string;
  marketStatus: MarketStatus;
  finalOutcome: Outcome | null;
}

export interface AdminState {
  treasury: string;
  supportedCollaterals: string[];
  feeConfig: FeeConfig;
  moduleAddresses: Record<string, string>;
  approvedCreators: string[];
  councilMembers: string[];
}

export interface WalletPermissions {
  address: string;
  canCreate: boolean;
  canResolve: boolean;
  canCouncil: boolean;
  canAdmin: boolean;
  isApprovedCreator: boolean;
  isCouncilMember: boolean;
  isAdmin: boolean;
}

export type TxStatus =
  | "idle"
  | "building"
  | "simulating"
  | "awaiting-signature"
  | "submitting"
  | "pending"
  | "success"
  | "failed";

export interface TxState {
  status: TxStatus;
  hash: string | null;
  error: string | null;
}

export type DikeErrorCode =
  | "SlippageExceeded"
  | "DeadlineExpired"
  | "InvalidStatus"
  | "UnsupportedCollateral"
  | "EncumberedPosition"
  | "InsufficientBalance"
  | "Unauthorized"
  | "Unknown";

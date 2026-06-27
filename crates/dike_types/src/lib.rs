#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, BytesN, String};

pub type MarketId = u64;
pub type PoolId = u64;
pub type RequestId = u64;
pub type CaseId = u64;
pub type ActionId = u64;

pub const BPS_DENOMINATOR: i128 = 10_000;
pub const DEFAULT_TRADE_FEE_BPS: u32 = 200;
pub const DEFAULT_LP_FEE_SHARE_BPS: u32 = 7_000;
pub const DEFAULT_TREASURY_FEE_SHARE_BPS: u32 = 2_000;
pub const DEFAULT_COD_FEE_SHARE_BPS: u32 = 1_000;
pub const DEFAULT_WINNER_BOND_SHARE_BPS: u32 = 6_000;
pub const DEFAULT_COUNCIL_BOND_SHARE_BPS: u32 = 3_000;
pub const DEFAULT_TREASURY_BOND_SHARE_BPS: u32 = 1_000;

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Yes,
    No,
    Invalid,
}

impl Outcome {
    pub fn unset() -> Self {
        Self::Invalid
    }
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    Created,
    Live,
    Paused,
    TradingClosed,
    ResolutionRequested,
    Proposed,
    Disputed,
    CouncilVoting,
    Resolved,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleStatus {
    None,
    Requested,
    Proposed,
    Disputed,
    Escalated,
    Finalized,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CouncilCaseStatus {
    Opened,
    CommitPhase,
    RevealPhase,
    ReadyToFinalize,
    Finalized,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelockActionKind {
    FeeConfig,
    Treasury,
    SupportedCollateral,
    Creator,
    CouncilMember,
    ModuleAddress,
    Pause,
    Upgrade,
}

#[contracttype]
#[derive(Clone)]
pub struct FeeConfig {
    pub trading_fee_bps: u32,
    pub lp_fee_share_bps: u32,
    pub treasury_fee_share_bps: u32,
    pub cod_fee_share_bps: u32,
    pub proposal_reward: i128,
    pub dispute_reward: i128,
    pub council_reward: i128,
    pub creation_fee: i128,
}

impl FeeConfig {
    pub fn default() -> Self {
        Self {
            trading_fee_bps: DEFAULT_TRADE_FEE_BPS,
            lp_fee_share_bps: DEFAULT_LP_FEE_SHARE_BPS,
            treasury_fee_share_bps: DEFAULT_TREASURY_FEE_SHARE_BPS,
            cod_fee_share_bps: DEFAULT_COD_FEE_SHARE_BPS,
            proposal_reward: 0,
            dispute_reward: 0,
            council_reward: 0,
            creation_fee: 0,
        }
    }
}

#[contracttype]
#[derive(Clone)]
pub struct MarketConfig {
    pub question: String,
    pub question_hash: BytesN<32>,
    pub rules_uri: String,
    pub rules_hash: BytesN<32>,
    pub expiry: u64,
    pub collateral: Address,
    pub bond_amount: i128,
    pub dispute_window: u64,
    pub category: String,
    pub creator: Address,
    pub fee_config: FeeConfig,
}

#[contracttype]
#[derive(Clone)]
pub struct MarketData {
    pub id: MarketId,
    pub question: String,
    pub question_hash: BytesN<32>,
    pub rules_uri: String,
    pub rules_hash: BytesN<32>,
    pub creator: Address,
    pub collateral: Address,
    pub yes_token_id: u64,
    pub no_token_id: u64,
    pub expiry: u64,
    pub status: MarketStatus,
    pub has_final_outcome: bool,
    pub final_outcome: Outcome,
    pub pool_id: PoolId,
    pub bond_amount: i128,
    pub dispute_window: u64,
    pub has_request: bool,
    pub request_id: RequestId,
    pub created_at: u64,
    pub fee_config: FeeConfig,
}

#[contracttype]
#[derive(Clone)]
pub struct VaultAccounting {
    pub total_deposited: i128,
    pub collateral_backing: i128,
    pub amm_collateral: i128,
    pub redeemed: i128,
    pub protocol_fees: i128,
    pub lp_fees: i128,
    pub cod_fees: i128,
    pub proposal_bonds: i128,
    pub dispute_bonds: i128,
    pub refundable: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct PoolData {
    pub id: PoolId,
    pub market_id: MarketId,
    pub yes_reserve: i128,
    pub no_reserve: i128,
    pub total_lp_shares: i128,
    pub accumulated_lp_fees: i128,
    pub accumulated_protocol_fees: i128,
    pub accumulated_cod_fees: i128,
    pub live: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct TradeQuote {
    pub amount_in: i128,
    pub fee: i128,
    pub net_in: i128,
    pub amount_out: i128,
    pub average_price_bps: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct ResolutionRequest {
    pub id: RequestId,
    pub market_id: MarketId,
    pub question_hash: BytesN<32>,
    pub rules_uri: String,
    pub expiry: u64,
    pub requested_at: u64,
    pub bond_amount: i128,
    pub dispute_window: u64,
    pub has_proposal: bool,
    pub proposer: Address,
    pub proposed_outcome: Outcome,
    pub proposal_evidence_uri: String,
    pub proposed_at: u64,
    pub has_dispute: bool,
    pub disputer: Address,
    pub disputed_outcome: Outcome,
    pub dispute_evidence_uri: String,
    pub disputed_at: u64,
    pub status: OracleStatus,
    pub has_final_outcome: bool,
    pub final_outcome: Outcome,
}

#[contracttype]
#[derive(Clone)]
pub struct OpenCaseConfig {
    pub proposal_bond: i128,
    pub dispute_bond: i128,
    pub commit_duration: u64,
    pub reveal_duration: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CouncilCase {
    pub id: CaseId,
    pub request_id: RequestId,
    pub market_id: MarketId,
    pub proposer: Address,
    pub proposer_outcome: Outcome,
    pub proposer_evidence_uri: String,
    pub disputer: Address,
    pub disputer_outcome: Outcome,
    pub disputer_evidence_uri: String,
    pub proposal_bond: i128,
    pub dispute_bond: i128,
    pub voting_start: u64,
    pub commit_end: u64,
    pub reveal_end: u64,
    pub status: CouncilCaseStatus,
    pub has_final_outcome: bool,
    pub final_outcome: Outcome,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub invalid_votes: u32,
    pub total_valid_votes: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct TimelockAction {
    pub id: ActionId,
    pub kind: TimelockActionKind,
    pub target: Address,
    pub payload_hash: BytesN<32>,
    pub execute_after: u64,
    pub expires_at: u64,
    pub executed: bool,
    pub cancelled: bool,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DikeError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    InvalidInput = 5,
    InvalidStatus = 6,
    InvalidTransition = 7,
    MarketExists = 8,
    MarketNotFound = 9,
    PoolNotFound = 10,
    RequestNotFound = 11,
    CaseNotFound = 12,
    AlreadyResolved = 13,
    AlreadyRedeemed = 14,
    InsufficientBalance = 15,
    InsufficientCollateral = 16,
    SlippageExceeded = 17,
    DeadlineExpired = 18,
    NotExpired = 19,
    DisputeWindowOpen = 20,
    DisputeWindowClosed = 21,
    EvidenceRequired = 22,
    AlreadyDisputed = 23,
    InvalidReveal = 24,
    VoteAlreadyCommitted = 25,
    VoteNotCommitted = 26,
    TooEarly = 27,
    TimelockNotReady = 28,
    ActionConsumed = 29,
    UnsupportedCollateral = 30,
    CreatorNotApproved = 31,
    ArithmeticError = 32,
}

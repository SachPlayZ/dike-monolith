#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, BytesN, String, Symbol};

pub const BPS_DENOMINATOR: i128 = 10_000;
pub const DEFAULT_TRADE_FEE_BPS: u32 = 200;
pub const DEFAULT_LP_FEE_SHARE_BPS: u32 = 7_000;
pub const DEFAULT_TREASURY_FEE_SHARE_BPS: u32 = 2_000;
pub const DEFAULT_COD_FEE_SHARE_BPS: u32 = 1_000;
pub const DEFAULT_WINNER_BOND_SHARE_BPS: u32 = 6_000;
pub const DEFAULT_COUNCIL_BOND_SHARE_BPS: u32 = 3_000;
pub const DEFAULT_TREASURY_BOND_SHARE_BPS: u32 = 1_000;
pub const DEFAULT_CHILD_COLLATERAL_BPS: u32 = 6_000;

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
    Timelock,
}

#[contracttype]
#[derive(Clone)]
pub struct FeeConfig {
    pub trading_fee_bps: u32,
    pub lp_fee_share_bps: u32,
    pub treasury_fee_share_bps: u32,
    pub cod_fee_share_bps: u32,
    pub council_reward: i128,
    pub creation_fee: i128,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            trading_fee_bps: DEFAULT_TRADE_FEE_BPS,
            lp_fee_share_bps: DEFAULT_LP_FEE_SHARE_BPS,
            treasury_fee_share_bps: DEFAULT_TREASURY_FEE_SHARE_BPS,
            cod_fee_share_bps: DEFAULT_COD_FEE_SHARE_BPS,
            council_reward: 0,
            creation_fee: 0,
        }
    }
}

/// Shared validation for `FeeConfig`.  All three share percentages must sum to
/// exactly 10 000 bps, trading fee must not exceed 10 % (1 000 bps), and
/// `council_reward` / `creation_fee` must be non-negative.
pub fn validate_fee_config(config: &FeeConfig) -> Result<(), DikeError> {
    let share_total = config.lp_fee_share_bps as u64
        + config.treasury_fee_share_bps as u64
        + config.cod_fee_share_bps as u64;
    if share_total != 10_000 || config.trading_fee_bps > 1_000 {
        return Err(DikeError::InvalidInput);
    }
    if config.council_reward < 0 || config.creation_fee < 0 {
        return Err(DikeError::InvalidAmount);
    }
    Ok(())
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
    pub id: u64,
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
    pub pool_id: u64,
    pub bond_amount: i128,
    pub dispute_window: u64,
    pub has_request: bool,
    pub request_id: u64,
    pub created_at: u64,
    pub fee_config: FeeConfig,
}

#[contracttype]
#[derive(Clone)]
pub struct VaultAccounting {
    pub total_deposited: i128,
    pub collateral_backing: i128,
    pub amm_collateral: i128,
    pub child_collateral_issued: i128,
    pub child_collateral_repaid: i128,
    pub child_collateral_defaulted: i128,
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
    pub id: u64,
    pub market_id: u64,
    pub yes_reserve: i128,
    pub no_reserve: i128,
    pub total_lp_shares: i128,
    pub accumulated_lp_fees: i128,
    pub accumulated_protocol_fees: i128,
    pub accumulated_cod_fees: i128,
    pub live: bool,
    pub fee_per_share_scaled: i128,
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
    pub id: u64,
    pub market_id: u64,
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
    pub token: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct CouncilCase {
    pub id: u64,
    pub request_id: u64,
    pub market_id: u64,
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
pub enum TimelockPayload {
    Treasury(Address),
    Creator(Address, bool),
    CouncilMember(Address, bool),
    SupportedCollateral(Address, bool),
    ModuleAddress(Symbol, Address),
    Pause(Address),
    FeeConfig(FeeConfig),
    Upgrade(Symbol, BytesN<32>),
    Timelock(Address),
}

impl TimelockPayload {
    /// `kind` is caller-supplied at `queue()` time purely for off-chain
    /// observability (it's what shows up in `ActionQueued`/`ActionExecuted`
    /// events). `execute()` dispatches on `payload`'s variant alone, so a
    /// mismatched `kind` would let a proposer label a dangerous payload
    /// (e.g. `Timelock(attacker)`) under a benign-looking kind (e.g.
    /// `Creator`). Reject that at `queue()` time.
    pub fn matches_kind(&self, kind: &TimelockActionKind) -> bool {
        matches!(
            (self, kind),
            (TimelockPayload::Treasury(_), TimelockActionKind::Treasury)
                | (TimelockPayload::Creator(_, _), TimelockActionKind::Creator)
                | (
                    TimelockPayload::CouncilMember(_, _),
                    TimelockActionKind::CouncilMember
                )
                | (
                    TimelockPayload::SupportedCollateral(_, _),
                    TimelockActionKind::SupportedCollateral
                )
                | (
                    TimelockPayload::ModuleAddress(_, _),
                    TimelockActionKind::ModuleAddress
                )
                | (TimelockPayload::Pause(_), TimelockActionKind::Pause)
                | (TimelockPayload::FeeConfig(_), TimelockActionKind::FeeConfig)
                | (TimelockPayload::Upgrade(_, _), TimelockActionKind::Upgrade)
                | (TimelockPayload::Timelock(_), TimelockActionKind::Timelock)
        )
    }
}

#[contracttype]
#[derive(Clone)]
pub struct TimelockAction {
    pub id: u64,
    pub kind: TimelockActionKind,
    pub target: Address,
    pub payload: TimelockPayload,
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
    ChainDepthExceeded = 33,
    ChildCollateralLimitExceeded = 34,
    EncumberedPosition = 35,
}

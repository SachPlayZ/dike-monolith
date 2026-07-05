#![no_std]

use dike_math::{
    bps, checked_add, checked_div, checked_mul, checked_sub, collateral_limit, invalid_refund,
};
use dike_types::{
    DikeError, MarketData, MarketStatus, Outcome, VaultAccounting, DEFAULT_CHILD_COLLATERAL_BPS,
};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short,
    token::Client as TokenClient, Address, BytesN, Env, Symbol, Vec,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;
/// Share of swept protocol fees held back into the insurance reserve instead
/// of paid to treasury — funds the default backstop. Hardcoded for now
/// (deferred exposure-cap/policy-knob work would make this governance-settable).
const INSURANCE_RESERVE_BPS: u32 = 2_000;
/// Cut of liquidation sale proceeds paid to whichever keeper calls
/// `liquidate_release`, taken off the top before debt repayment so a keeper
/// is always paid for a valid liquidation regardless of how underwater the
/// position is. Hardcoded for now, same reasoning as INSURANCE_RESERVE_BPS.
const LIQUIDATION_BONUS_BPS: u32 = 300;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Treasury,
    Accounting(u64),
    UserDeposit(u64, Address),
    RootStake(u64, Address, Outcome),
    ChildCollateralUsedByOutcome(u64, Address, Outcome),
    ChildCollateralUsed(u64, Address),
    ChildParent(u64, Address),
    ChildParentOutcome(u64, Address),
    ChildLoan(u64, u64, Address),
    ChildLoanByOutcome(u64, Outcome, u64, Outcome, Address),
    ChildDebt(u64, Address),
    ParentDebt(u64, Address, Outcome),
    ChildMarketsOf(u64, Address, Outcome),
    InsuranceReserve,
    Bond(u64, Address, bool),
    BondToken(u64, Address, bool),
    BondMarket(u64, Address, bool),
    Redeemed(u64, Address, Outcome),
    InvalidDustCarry(u64),
    Paused,
}

#[contractclient(name = "DikeTokensClient")]
pub trait DikeTokens {
    fn burn_for_redeem(
        env: Env,
        owner: Address,
        market_id: u64,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError>;
}

#[contractclient(name = "DikeRegistryClient")]
pub trait DikeRegistry {
    fn get_final_outcome(env: Env, market_id: u64) -> Result<Outcome, DikeError>;
    fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError>;
}

#[contractevent(topics = ["role"], data_format = "single-value")]
#[derive(Clone)]
pub struct RoleSet {
    #[topic]
    pub role: Symbol,
    pub module: Address,
}

#[contractevent(topics = ["admin"], data_format = "single-value")]
#[derive(Clone)]
pub struct AdminSet {
    pub admin: Address,
}

#[contractevent(topics = ["treas"], data_format = "single-value")]
#[derive(Clone)]
pub struct TreasurySet {
    pub treasury: Address,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent(topics = ["deposit"], data_format = "single-value")]
#[derive(Clone)]
pub struct MarketDeposit {
    #[topic]
    pub market_id: u64,
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["cfund"], data_format = "vec")]
#[derive(Clone)]
pub struct ChildPredictionFunded {
    #[topic]
    pub parent_market_id: u64,
    #[topic]
    pub child_market_id: u64,
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["cpay"], data_format = "vec")]
#[derive(Clone)]
pub struct ChildCollateralRepaid {
    #[topic]
    pub parent_market_id: u64,
    #[topic]
    pub child_market_id: u64,
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["liq_settle"], data_format = "vec")]
#[derive(Clone)]
pub struct LiquidationSettled {
    #[topic]
    pub parent_market_id: u64,
    #[topic]
    pub user: Address,
    pub liquidator: Address,
    pub proceeds: i128,
    pub debt_repaid: i128,
    pub bonus: i128,
    pub remainder: i128,
}

#[contractevent(topics = ["redeem"], data_format = "vec")]
#[derive(Clone)]
pub struct Redeemed {
    #[topic]
    pub market_id: u64,
    #[topic]
    pub user: Address,
    pub outcome: Outcome,
    pub payout: i128,
}

#[contractevent(topics = ["bond"], data_format = "vec")]
#[derive(Clone)]
pub struct BondLocked {
    #[topic]
    pub request_id: u64,
    #[topic]
    pub user: Address,
    pub amount: i128,
    pub is_dispute: bool,
}

#[contractevent(topics = ["bond_rel"], data_format = "single-value")]
#[derive(Clone)]
pub struct BondReleased {
    #[topic]
    pub request_id: u64,
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contractevent(topics = ["fee"], data_format = "vec")]
#[derive(Clone)]
pub struct FeesCollected {
    #[topic]
    pub market_id: u64,
    pub lp_fee: i128,
    pub protocol_fee: i128,
    pub cod_fee: i128,
}

#[contract]
pub struct CollateralVault;

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);
}

fn zero_accounting() -> VaultAccounting {
    VaultAccounting {
        total_deposited: 0,
        collateral_backing: 0,
        amm_collateral: 0,
        child_collateral_issued: 0,
        child_collateral_repaid: 0,
        child_collateral_defaulted: 0,
        redeemed: 0,
        protocol_fees: 0,
        lp_fees: 0,
        cod_fees: 0,
        proposal_bonds: 0,
        dispute_bonds: 0,
        refundable: 0,
        shortfall: 0,
    }
}

fn require_admin(env: &Env) -> Result<(), DikeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(DikeError::NotInitialized)?;
    admin.require_auth();
    Ok(())
}

fn require_role(env: &Env, role: Symbol) -> Result<(), DikeError> {
    let module: Address = env
        .storage()
        .instance()
        .get(&DataKey::Role(role))
        .ok_or(DikeError::Unauthorized)?;
    module.require_auth();
    Ok(())
}

fn read_role(env: &Env, role: Symbol) -> Result<Address, DikeError> {
    env.storage()
        .instance()
        .get(&DataKey::Role(role))
        .ok_or(DikeError::Unauthorized)
}

fn read_accounting(env: &Env, market_id: u64) -> VaultAccounting {
    let key = DataKey::Accounting(market_id);
    if !env.storage().persistent().has(&key) {
        return zero_accounting();
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(zero_accounting)
}

fn write_accounting(env: &Env, market_id: u64, accounting: &VaultAccounting) {
    let key = DataKey::Accounting(market_id);
    env.storage().persistent().set(&key, accounting);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn read_i128(env: &Env, key: &DataKey) -> i128 {
    if !env.storage().persistent().has(key) {
        return 0;
    }
    env.storage()
        .persistent()
        .extend_ttl(key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(key).unwrap_or(0)
}

fn write_i128(env: &Env, key: &DataKey, amount: i128) {
    env.storage().persistent().set(key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(key, MIN_TTL, EXTEND_TTL);
}

fn saturating_sub_i128(env: &Env, key: &DataKey, amount: i128) -> Result<i128, DikeError> {
    let current = read_i128(env, key);
    let spent = if current > amount { amount } else { current };
    write_i128(env, key, checked_sub(current, spent)?);
    Ok(spent)
}

fn read_parent(env: &Env, child_market_id: u64, user: Address) -> u64 {
    let key = DataKey::ChildParent(child_market_id, user);
    if !env.storage().persistent().has(&key) {
        return 0;
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(&key).unwrap_or(0)
}

fn write_parent(env: &Env, child_market_id: u64, user: Address, parent_market_id: u64) {
    let key = DataKey::ChildParent(child_market_id, user);
    env.storage().persistent().set(&key, &parent_market_id);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn read_parent_outcome(env: &Env, child_market_id: u64, user: Address) -> Outcome {
    let key = DataKey::ChildParentOutcome(child_market_id, user);
    if !env.storage().persistent().has(&key) {
        return Outcome::Invalid;
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Outcome::Invalid)
}

fn write_parent_outcome(env: &Env, child_market_id: u64, user: Address, outcome: Outcome) {
    let key = DataKey::ChildParentOutcome(child_market_id, user);
    env.storage().persistent().set(&key, &outcome);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn read_child_markets_of(
    env: &Env,
    parent_market_id: u64,
    user: Address,
    outcome: Outcome,
) -> Vec<u64> {
    let key = DataKey::ChildMarketsOf(parent_market_id, user, outcome);
    if !env.storage().persistent().has(&key) {
        return Vec::new(env);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env))
}

fn record_child_market(
    env: &Env,
    parent_market_id: u64,
    user: Address,
    outcome: Outcome,
    child_market_id: u64,
) {
    let key = DataKey::ChildMarketsOf(parent_market_id, user.clone(), outcome);
    let mut children = read_child_markets_of(env, parent_market_id, user, outcome);
    if !children.contains(child_market_id) {
        children.push_back(child_market_id);
    }
    env.storage().persistent().set(&key, &children);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn user_deposit_key(market_id: u64, user: Address) -> DataKey {
    DataKey::UserDeposit(market_id, user)
}

fn child_used_key(parent_market_id: u64, user: Address) -> DataKey {
    DataKey::ChildCollateralUsed(parent_market_id, user)
}

fn child_loan_key(parent_market_id: u64, child_market_id: u64, user: Address) -> DataKey {
    DataKey::ChildLoan(parent_market_id, child_market_id, user)
}

fn root_stake_key(market_id: u64, user: Address, outcome: Outcome) -> DataKey {
    DataKey::RootStake(market_id, user, outcome)
}

fn child_used_outcome_key(parent_market_id: u64, user: Address, outcome: Outcome) -> DataKey {
    DataKey::ChildCollateralUsedByOutcome(parent_market_id, user, outcome)
}

fn child_loan_outcome_key(
    parent_market_id: u64,
    parent_outcome: Outcome,
    child_market_id: u64,
    child_outcome: Outcome,
    user: Address,
) -> DataKey {
    DataKey::ChildLoanByOutcome(
        parent_market_id,
        parent_outcome,
        child_market_id,
        child_outcome,
        user,
    )
}

fn child_debt_key(child_market_id: u64, user: Address) -> DataKey {
    DataKey::ChildDebt(child_market_id, user)
}

fn parent_debt_key(parent_market_id: u64, user: Address, outcome: Outcome) -> DataKey {
    DataKey::ParentDebt(parent_market_id, user, outcome)
}

/// Pays down up to `repayment` of `child_market_id`'s outstanding debt (as a
/// child, owed upstream to its parent), capped by what's actually still owed
/// both here and upstream. Returns the amount actually applied. Restores the
/// parent's `collateral_backing`/`refundable` by that same amount — the
/// Gap-1-symmetric counterpart to `open_child_credit_for_trade`'s reallocation,
/// without which a fully-repaid loan would permanently strand parent backing.
/// Shared by `redeem_resolved`, `repay_child_collateral`, and liquidation.
fn settle_child_debt(
    env: &Env,
    child_market_id: u64,
    user: Address,
    repayment: i128,
) -> Result<i128, DikeError> {
    if repayment <= 0 {
        return Ok(0);
    }
    let debt_key = child_debt_key(child_market_id, user.clone());
    let raw_child_debt = read_i128(env, &debt_key);
    if raw_child_debt == 0 {
        return Ok(0);
    }
    let parent_market_id = read_parent(env, child_market_id, user.clone());
    let parent_outcome = read_parent_outcome(env, child_market_id, user.clone());

    let effective_debt = if parent_market_id != 0 {
        let upstream_debt = read_i128(
            env,
            &parent_debt_key(parent_market_id, user.clone(), parent_outcome),
        );
        if upstream_debt == 0 {
            // Upstream already cleared (e.g. via default resolution) — this
            // child's own debt record is stale, zero it and stop.
            write_i128(env, &debt_key, 0);
            return Ok(0);
        } else if upstream_debt < raw_child_debt {
            upstream_debt
        } else {
            raw_child_debt
        }
    } else {
        raw_child_debt
    };

    let applied = if repayment > effective_debt {
        effective_debt
    } else {
        repayment
    };
    if applied == 0 {
        return Ok(0);
    }

    write_i128(env, &debt_key, checked_sub(raw_child_debt, applied)?);
    if parent_market_id != 0 {
        let upstream_key = parent_debt_key(parent_market_id, user.clone(), parent_outcome);
        let upstream_debt = read_i128(env, &upstream_key);
        write_i128(env, &upstream_key, checked_sub(upstream_debt, applied)?);
        let _ = saturating_sub_i128(
            env,
            &child_used_outcome_key(parent_market_id, user.clone(), parent_outcome),
            applied,
        )?;
        let _ = saturating_sub_i128(
            env,
            &child_used_key(parent_market_id, user.clone()),
            applied,
        )?;

        let mut parent_accounting = read_accounting(env, parent_market_id);
        parent_accounting.collateral_backing =
            checked_add(parent_accounting.collateral_backing, applied)?;
        parent_accounting.refundable = checked_add(parent_accounting.refundable, applied)?;
        parent_accounting.child_collateral_repaid =
            checked_add(parent_accounting.child_collateral_repaid, applied)?;
        write_accounting(env, parent_market_id, &parent_accounting);
    }
    Ok(applied)
}

/// Caps a claim against what this market can actually still pay in real
/// money. `collateral_backing` is the real bucket that's actually here right
/// now (already reflects any insurance top-up); `shortfall` is the known
/// permanently-unrecoverable amount (set by `resolve_parent_default`) — so
/// `collateral_backing + shortfall` is what the market's claims would total
/// if nothing had ever defaulted. Once shortfall is nonzero, every claim gets
/// scaled by the SAME ratio `collateral_backing / (collateral_backing +
/// shortfall)`, computed fresh each call so it's identical for the first
/// redeemer and the last. That's what makes this a pro-rata haircut instead
/// of a first-come-first-served race: whoever redeems first no longer gets
/// paid in full while whoever redeems last discovers `InsufficientCollateral`
/// and gets nothing — everyone gets the same proportional share.
///
/// Returns the real amount payable; the caller must credit
/// `accounting.redeemed`/`.refundable` by exactly this returned amount (not
/// the original claim) so those fields keep tracking real money, not claims.
fn capped_payout(env: &Env, market_id: u64, gross_payout: i128) -> Result<i128, DikeError> {
    if gross_payout <= 0 {
        return Ok(0);
    }
    let accounting = read_accounting(env, market_id);
    if accounting.collateral_backing <= 0 {
        return Ok(0);
    }
    let remaining = checked_sub(accounting.collateral_backing, accounting.redeemed)?;
    if remaining <= 0 {
        return Ok(0);
    }
    let claim = if accounting.shortfall == 0 {
        gross_payout
    } else {
        let total_claims = checked_add(accounting.collateral_backing, accounting.shortfall)?;
        checked_div(
            checked_mul(gross_payout, accounting.collateral_backing)?,
            total_claims,
        )?
    };
    Ok(if claim > remaining { remaining } else { claim })
}

fn read_insurance_reserve(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::InsuranceReserve)
        .unwrap_or(0)
}

fn write_insurance_reserve(env: &Env, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::InsuranceReserve, &amount);
}

/// Called when a parent-side redeemer's outcome loses (or a parent gets
/// cancelled) while it's still backing live child credit — `total_defaulted`
/// is the amount that can no longer be repaid by the user themselves.
///
/// The victim of a parent default is the PARENT market's own remaining
/// redeemers, not the child: the child market got real money upfront at
/// credit-open time (Gap 1's reallocation) and keeps it forever regardless of
/// what happens to the parent — exactly like a loan against a house doesn't
/// put the thing you bought with it at risk if you stop paying the mortgage,
/// it puts the *lender's* balance sheet at risk. So this tops up the
/// PARENT's own `collateral_backing` from the insurance reserve first, and
/// whatever the reserve can't cover becomes the parent's own `shortfall` —
/// which `capped_payout` then applies as a pro-rata haircut to every future
/// redemption of the parent market's winning side, instead of a silent
/// dashboard counter or a hard revert for whoever redeems last.
///
/// Still walks `child_markets_of` to clear the now-uncollectable debt's
/// bookkeeping on the child side (`child_debt`/`child_used_*` keys) — not
/// because the child's money is at risk, but so its debt view goes back to
/// zero and `assert_position_transfer_allowed` stops blocking that position.
fn resolve_parent_default(
    env: &Env,
    parent_market_id: u64,
    user: Address,
    parent_outcome: Outcome,
    total_defaulted: i128,
) -> Result<(), DikeError> {
    if total_defaulted <= 0 {
        return Ok(());
    }
    let mut parent_accounting = read_accounting(env, parent_market_id);
    parent_accounting.child_collateral_defaulted = checked_add(
        parent_accounting.child_collateral_defaulted,
        total_defaulted,
    )?;

    let mut reserve = read_insurance_reserve(env);
    let reserve_draw = if total_defaulted > reserve {
        reserve
    } else {
        total_defaulted
    };
    let unrecoverable = checked_sub(total_defaulted, reserve_draw)?;
    if reserve_draw > 0 {
        reserve = checked_sub(reserve, reserve_draw)?;
        parent_accounting.collateral_backing =
            checked_add(parent_accounting.collateral_backing, reserve_draw)?;
        parent_accounting.refundable = checked_add(parent_accounting.refundable, reserve_draw)?;
    }
    if unrecoverable > 0 {
        parent_accounting.shortfall = checked_add(parent_accounting.shortfall, unrecoverable)?;
    }
    write_accounting(env, parent_market_id, &parent_accounting);
    write_insurance_reserve(env, reserve);

    // The debt itself is now resolved (insurance-covered or written off as
    // shortfall) — clear it so it doesn't sit there permanently marked owed
    // with no path to ever collect it.
    write_i128(
        env,
        &parent_debt_key(parent_market_id, user.clone(), parent_outcome),
        0,
    );

    let mut remaining = total_defaulted;
    let children = read_child_markets_of(env, parent_market_id, user.clone(), parent_outcome);
    for child_market_id in children.iter() {
        if remaining == 0 {
            break;
        }
        let debt_key = child_debt_key(child_market_id, user.clone());
        let owed = read_i128(env, &debt_key);
        if owed == 0 {
            continue;
        }
        let cleared = if owed > remaining { remaining } else { owed };
        write_i128(env, &debt_key, checked_sub(owed, cleared)?);
        let _ = saturating_sub_i128(
            env,
            &child_used_outcome_key(parent_market_id, user.clone(), parent_outcome),
            cleared,
        )?;
        let _ = saturating_sub_i128(
            env,
            &child_used_key(parent_market_id, user.clone()),
            cleared,
        )?;
        remaining = checked_sub(remaining, cleared)?;
    }
    Ok(())
}

fn market_collateral(env: &Env, market_id: u64) -> Result<Address, DikeError> {
    let registry = read_role(env, symbol_short!("registry"))?;
    Ok(DikeRegistryClient::new(env, &registry)
        .get_market(&market_id)
        .collateral)
}

fn market_data(env: &Env, market_id: u64) -> Result<MarketData, DikeError> {
    let registry = read_role(env, symbol_short!("registry"))?;
    Ok(DikeRegistryClient::new(env, &registry).get_market(&market_id))
}

fn require_market_collateral(env: &Env, token: &Address, market_id: u64) -> Result<(), DikeError> {
    if *token != market_collateral(env, market_id)? {
        return Err(DikeError::UnsupportedCollateral);
    }
    Ok(())
}

fn transfer_token(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    let client = TokenClient::new(env, token);
    client.transfer(from, to, &amount);
}

fn add_redeemed(
    env: &Env,
    market_id: u64,
    user: Address,
    outcome: Outcome,
    amount: i128,
) -> Result<(), DikeError> {
    let key = DataKey::Redeemed(market_id, user, outcome);
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    let next = checked_add(current, amount)?;
    env.storage().persistent().set(&key, &next);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    Ok(())
}

#[contractimpl]
impl CollateralVault {
    pub fn __constructor(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::Paused, &false);
        bump(&env);
    }

    pub fn set_admin(env: Env, admin: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        AdminSet { admin }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Ok(())
    }

    pub fn set_role(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Role(role.clone()), &module);
        RoleSet { role, module }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn set_treasury(env: Env, treasury: Address) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        TreasurySet { treasury }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        Paused { paused }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn deposit_for_market(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(DikeError::InvalidStatus);
        }
        require_market_collateral(&env, &token, market_id)?;
        transfer_token(&env, &token, &user, &env.current_contract_address(), amount);
        let mut accounting = read_accounting(&env, market_id);
        accounting.total_deposited = checked_add(accounting.total_deposited, amount)?;
        accounting.collateral_backing = checked_add(accounting.collateral_backing, amount)?;
        accounting.refundable = checked_add(accounting.refundable, amount)?;
        write_accounting(&env, market_id, &accounting);
        let deposit_key = user_deposit_key(market_id, user.clone());
        write_i128(
            &env,
            &deposit_key,
            checked_add(read_i128(&env, &deposit_key), amount)?,
        );
        MarketDeposit {
            market_id,
            user,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn record_cash_stake(
        env: Env,
        user: Address,
        market_id: u64,
        outcome: Outcome,
        collateral_in: i128,
        tokens_out: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if collateral_in <= 0 || tokens_out <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let stake_key = root_stake_key(market_id, user.clone(), outcome);
        write_i128(
            &env,
            &stake_key,
            checked_add(read_i128(&env, &stake_key), collateral_in)?,
        );
        Ok(())
    }

    pub fn fund_child_prediction(
        env: Env,
        _user: Address,
        _parent_market_id: u64,
        _child_market_id: u64,
        _amount: i128,
    ) -> Result<(), DikeError> {
        let _ = env;
        Err(DikeError::Unauthorized)
    }

    pub fn open_child_credit_for_trade(
        env: Env,
        user: Address,
        parent_market_id: u64,
        parent_outcome: Outcome,
        child_market_id: u64,
        child_outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        if parent_market_id == child_market_id {
            return Err(DikeError::InvalidInput);
        }
        let parent_market = market_data(&env, parent_market_id)?;
        if parent_market.status != MarketStatus::Live {
            return Err(DikeError::InvalidStatus);
        }
        if read_parent(&env, parent_market_id, user.clone()) != 0 {
            return Err(DikeError::ChainDepthExceeded);
        }
        let existing_parent = read_parent(&env, child_market_id, user.clone());
        if existing_parent != 0 && existing_parent != parent_market_id {
            return Err(DikeError::InvalidInput);
        }

        let stake = read_i128(
            &env,
            &root_stake_key(parent_market_id, user.clone(), parent_outcome),
        );
        let limit = collateral_limit(stake, DEFAULT_CHILD_COLLATERAL_BPS)?;
        let used_key = child_used_outcome_key(parent_market_id, user.clone(), parent_outcome);
        let used = read_i128(&env, &used_key);
        let next_used = checked_add(used, amount)?;
        if next_used > limit {
            return Err(DikeError::ChildCollateralLimitExceeded);
        }

        write_i128(&env, &used_key, next_used);
        let legacy_used_key = child_used_key(parent_market_id, user.clone());
        write_i128(
            &env,
            &legacy_used_key,
            checked_add(read_i128(&env, &legacy_used_key), amount)?,
        );

        let loan_key = child_loan_outcome_key(
            parent_market_id,
            parent_outcome,
            child_market_id,
            child_outcome,
            user.clone(),
        );
        write_i128(
            &env,
            &loan_key,
            checked_add(read_i128(&env, &loan_key), amount)?,
        );
        let legacy_loan_key = child_loan_key(parent_market_id, child_market_id, user.clone());
        write_i128(
            &env,
            &legacy_loan_key,
            checked_add(read_i128(&env, &legacy_loan_key), amount)?,
        );
        let debt_key = child_debt_key(child_market_id, user.clone());
        write_i128(
            &env,
            &debt_key,
            checked_add(read_i128(&env, &debt_key), amount)?,
        );
        let parent_debt_key = parent_debt_key(parent_market_id, user.clone(), parent_outcome);
        write_i128(
            &env,
            &parent_debt_key,
            checked_add(read_i128(&env, &parent_debt_key), amount)?,
        );
        if existing_parent == 0 {
            write_parent(&env, child_market_id, user.clone(), parent_market_id);
            write_parent_outcome(&env, child_market_id, user.clone(), parent_outcome);
        }
        record_child_market(
            &env,
            parent_market_id,
            user.clone(),
            parent_outcome,
            child_market_id,
        );

        // Credit is a reallocation of the parent's own real backing, not a new
        // claim conjured against the shared vault balance — without this the
        // child market gets a fully redeemable claim backed by nothing.
        let mut parent_accounting = read_accounting(&env, parent_market_id);
        if parent_accounting.collateral_backing < amount || parent_accounting.refundable < amount {
            return Err(DikeError::InsufficientCollateral);
        }
        parent_accounting.collateral_backing =
            checked_sub(parent_accounting.collateral_backing, amount)?;
        parent_accounting.refundable = checked_sub(parent_accounting.refundable, amount)?;
        parent_accounting.child_collateral_issued =
            checked_add(parent_accounting.child_collateral_issued, amount)?;
        write_accounting(&env, parent_market_id, &parent_accounting);

        let mut child_accounting = read_accounting(&env, child_market_id);
        child_accounting.total_deposited = checked_add(child_accounting.total_deposited, amount)?;
        child_accounting.collateral_backing =
            checked_add(child_accounting.collateral_backing, amount)?;
        child_accounting.refundable = checked_add(child_accounting.refundable, amount)?;
        write_accounting(&env, child_market_id, &child_accounting);

        ChildPredictionFunded {
            parent_market_id,
            child_market_id,
            user,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Voluntary repayment of an outstanding child-market debt, ahead of that
    /// child market resolving. Previously unimplemented — there was no way for
    /// a user to close a risky credit line proactively; exposure just sat
    /// open until whichever market resolved first.
    pub fn repay_child_collateral(
        env: Env,
        token: Address,
        user: Address,
        child_market_id: u64,
        amount: i128,
    ) -> Result<i128, DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, child_market_id)?;
        let parent_market_id = read_parent(&env, child_market_id, user.clone());
        transfer_token(&env, &token, &user, &env.current_contract_address(), amount);
        let applied = settle_child_debt(&env, child_market_id, user.clone(), amount)?;
        let unused = checked_sub(amount, applied)?;
        if unused > 0 {
            transfer_token(&env, &token, &env.current_contract_address(), &user, unused);
        }
        ChildCollateralRepaid {
            parent_market_id,
            child_market_id,
            user,
            amount: applied,
        }
        .publish(&env);
        Ok(applied)
    }

    pub fn release_trade_payout(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        outcome: Outcome,
        tokens_sold: i128,
        payout: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if tokens_sold <= 0 || payout <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, market_id)?;
        if read_i128(&env, &child_debt_key(market_id, user.clone())) > 0
            || read_i128(&env, &parent_debt_key(market_id, user.clone(), outcome)) > 0
        {
            return Err(DikeError::EncumberedPosition);
        }
        let mut accounting = read_accounting(&env, market_id);
        if accounting.collateral_backing < payout || accounting.refundable < payout {
            return Err(DikeError::InsufficientCollateral);
        }
        let stake_key = root_stake_key(market_id, user.clone(), outcome);
        let deposit_key = user_deposit_key(market_id, user.clone());
        let _ = saturating_sub_i128(&env, &stake_key, tokens_sold)?;
        let _ = saturating_sub_i128(&env, &deposit_key, payout)?;
        accounting.collateral_backing = checked_sub(accounting.collateral_backing, payout)?;
        accounting.refundable = checked_sub(accounting.refundable, payout)?;
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
        Ok(())
    }

    /// Called by the AMM's `liquidate_position` after it force-sells a user's
    /// entire parent-outcome balance (a normal `sell()` would call
    /// `release_trade_payout` here and pay `proceeds` straight to the seller —
    /// this is the liquidation variant: extract `proceeds` from the parent's
    /// backing exactly like a normal sale would, then instead of paying the
    /// user, route it through debt settlement first).
    ///
    /// Order: liquidator bonus is taken off the top of `proceeds` so a keeper
    /// always gets paid for a valid call regardless of how underwater the
    /// position is (otherwise nobody would bother liquidating the worst
    /// positions — exactly the ones most urgent to close). What's left pays
    /// down debt across every market in `child_markets_of`, in list order,
    /// via `settle_child_debt` (which restores the parent's own backing as
    /// each debt gets repaid). Any remainder after that goes to the user.
    /// Whatever debt survives because proceeds ran short is handed to
    /// `resolve_parent_default` — same insurance-then-shortfall path a
    /// resolution-time default would take.
    pub fn liquidate_release(
        env: Env,
        token: Address,
        user: Address,
        liquidator: Address,
        parent_market_id: u64,
        parent_outcome: Outcome,
        tokens_sold: i128,
        proceeds: i128,
    ) -> Result<i128, DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if tokens_sold <= 0 || proceeds <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, parent_market_id)?;

        let total_used = read_i128(
            &env,
            &child_used_outcome_key(parent_market_id, user.clone(), parent_outcome),
        );
        if total_used == 0 {
            return Err(DikeError::InvalidInput);
        }

        let mut parent_accounting = read_accounting(&env, parent_market_id);
        if parent_accounting.collateral_backing < proceeds
            || parent_accounting.refundable < proceeds
        {
            return Err(DikeError::InsufficientCollateral);
        }
        let stake_key = root_stake_key(parent_market_id, user.clone(), parent_outcome);
        let deposit_key = user_deposit_key(parent_market_id, user.clone());
        let _ = saturating_sub_i128(&env, &stake_key, tokens_sold)?;
        let _ = saturating_sub_i128(&env, &deposit_key, proceeds)?;
        parent_accounting.collateral_backing =
            checked_sub(parent_accounting.collateral_backing, proceeds)?;
        parent_accounting.refundable = checked_sub(parent_accounting.refundable, proceeds)?;
        write_accounting(&env, parent_market_id, &parent_accounting);

        let bonus = bps(proceeds, LIQUIDATION_BONUS_BPS)?;
        let mut pool = checked_sub(proceeds, bonus)?;
        let mut debt_repaid = 0i128;
        let children = read_child_markets_of(&env, parent_market_id, user.clone(), parent_outcome);
        for child_market_id in children.iter() {
            if pool == 0 {
                break;
            }
            let applied = settle_child_debt(&env, child_market_id, user.clone(), pool)?;
            pool = checked_sub(pool, applied)?;
            debt_repaid = checked_add(debt_repaid, applied)?;
        }
        let remainder = pool;

        let unrecovered = read_i128(
            &env,
            &child_used_outcome_key(parent_market_id, user.clone(), parent_outcome),
        );
        if unrecovered > 0 {
            resolve_parent_default(
                &env,
                parent_market_id,
                user.clone(),
                parent_outcome,
                unrecovered,
            )?;
        }

        if bonus > 0 {
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &liquidator,
                bonus,
            );
        }
        if remainder > 0 {
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &user,
                remainder,
            );
        }

        LiquidationSettled {
            parent_market_id,
            user,
            liquidator,
            proceeds,
            debt_repaid,
            bonus,
            remainder,
        }
        .publish(&env);
        Ok(debt_repaid)
    }

    /// Called by the AMM's `liquidate_child_position` — the keeper's
    /// follow-up close for whatever child debt survived a parent liquidation
    /// (or simply never triggered a parent-side liquidation but the child
    /// leg itself is now underwater relative to its own debt). Symmetric to
    /// `liquidate_release` but scoped to a single child, since a child market
    /// can never itself be a parent (`ChainDepthExceeded` blocks chaining).
    pub fn liquidate_child_release(
        env: Env,
        token: Address,
        user: Address,
        liquidator: Address,
        child_market_id: u64,
        child_outcome: Outcome,
        tokens_sold: i128,
        proceeds: i128,
    ) -> Result<i128, DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if tokens_sold <= 0 || proceeds <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, child_market_id)?;

        let debt = read_i128(&env, &child_debt_key(child_market_id, user.clone()));
        if debt == 0 {
            return Err(DikeError::InvalidInput);
        }

        let mut child_accounting = read_accounting(&env, child_market_id);
        if child_accounting.collateral_backing < proceeds || child_accounting.refundable < proceeds
        {
            return Err(DikeError::InsufficientCollateral);
        }
        let stake_key = root_stake_key(child_market_id, user.clone(), child_outcome);
        let deposit_key = user_deposit_key(child_market_id, user.clone());
        let _ = saturating_sub_i128(&env, &stake_key, tokens_sold)?;
        let _ = saturating_sub_i128(&env, &deposit_key, proceeds)?;
        child_accounting.collateral_backing =
            checked_sub(child_accounting.collateral_backing, proceeds)?;
        child_accounting.refundable = checked_sub(child_accounting.refundable, proceeds)?;
        write_accounting(&env, child_market_id, &child_accounting);

        let bonus = bps(proceeds, LIQUIDATION_BONUS_BPS)?;
        let pool = checked_sub(proceeds, bonus)?;
        let applied = settle_child_debt(&env, child_market_id, user.clone(), pool)?;
        let remainder = checked_sub(pool, applied)?;

        // The position is now fully sold — if proceeds still didn't cover the
        // whole debt, there's nothing left to liquidate further. That
        // shortfall lands on the upstream parent's own book, same as any
        // other default.
        let still_owed = read_i128(&env, &child_debt_key(child_market_id, user.clone()));
        if still_owed > 0 {
            let parent_market_id = read_parent(&env, child_market_id, user.clone());
            let parent_outcome = read_parent_outcome(&env, child_market_id, user.clone());
            if parent_market_id != 0 {
                resolve_parent_default(
                    &env,
                    parent_market_id,
                    user.clone(),
                    parent_outcome,
                    still_owed,
                )?;
            }
        }

        if bonus > 0 {
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &liquidator,
                bonus,
            );
        }
        if remainder > 0 {
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &user,
                remainder,
            );
        }

        LiquidationSettled {
            parent_market_id: child_market_id,
            user,
            liquidator,
            proceeds,
            debt_repaid: applied,
            bonus,
            remainder,
        }
        .publish(&env);
        Ok(applied)
    }

    pub fn redeem(
        env: Env,
        _token: Address,
        _user: Address,
        _market_id: u64,
        _final_outcome: Outcome,
        _redeemed_outcome: Outcome,
        _amount: i128,
    ) -> Result<i128, DikeError> {
        let _ = env;
        Err(DikeError::Unauthorized)
    }

    pub fn redeem_resolved(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        redeemed_outcome: Outcome,
        amount: i128,
    ) -> Result<i128, DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }

        let registry = read_role(&env, symbol_short!("registry"))?;
        let tokens = read_role(&env, symbol_short!("tokens"))?;
        let registry_client = DikeRegistryClient::new(&env, &registry);
        let market = registry_client.get_market(&market_id);
        if token != market.collateral {
            return Err(DikeError::UnsupportedCollateral);
        }
        let final_outcome = registry_client.get_final_outcome(&market_id);

        let gross_payout = match final_outcome {
            Outcome::Invalid => {
                // Fold in per-market carry so odd-stroop dust is never lost.
                let carry_key = DataKey::InvalidDustCarry(market_id);
                let carry = read_i128(&env, &carry_key);
                let effective = checked_add(carry, amount)?;
                let refund = invalid_refund(effective)?;
                let new_carry = checked_sub(effective, checked_add(refund, refund)?)?;
                write_i128(&env, &carry_key, new_carry);
                refund
            }
            Outcome::Yes => {
                if redeemed_outcome == Outcome::Yes {
                    amount
                } else {
                    0
                }
            }
            Outcome::No => {
                if redeemed_outcome == Outcome::No {
                    amount
                } else {
                    0
                }
            }
        };

        add_redeemed(&env, market_id, user.clone(), redeemed_outcome, amount)?;

        // Debt settlement happens FIRST, against the full theoretical claim —
        // not the haircut-capped amount — because repaying debt is exactly
        // what RESTORES this market's own backing (see below), and capping
        // before that restoration would circularly punish the redemption
        // that's supposed to fix the shortfall in the first place.

        // This market redeeming may itself be a child (owes debt upstream) —
        // settle that first, it has priority on the claim. `settle_child_debt`
        // credits the UPSTREAM parent's backing (a different market) using
        // THIS market's own redemption proceeds as the source — unlike the
        // parent_repayment case below (which credits market_id itself, a
        // self-contained wash), this is a genuine transfer between two
        // different markets' books, so it needs a matching debit here on
        // market_id's own side or the upstream credit double-counts against
        // whatever's left unclaimed in market_id's bucket.
        let child_repayment = settle_child_debt(&env, market_id, user.clone(), gross_payout)?;
        if child_repayment > 0 {
            let mut this_accounting = read_accounting(&env, market_id);
            this_accounting.collateral_backing =
                checked_sub(this_accounting.collateral_backing, child_repayment)?;
            this_accounting.refundable = checked_sub(this_accounting.refundable, child_repayment)?;
            write_accounting(&env, market_id, &this_accounting);
        }

        // Independently, this market may be a parent that other children owe
        // debt to — withhold whatever's left of the claim against that, and
        // restore this market's OWN backing by the withheld amount (the
        // Gap-1-symmetric step for this direction: the redeemer is using
        // their own winning claim to repay their own debt, so the money
        // never leaves — it just stops being "on loan").
        let parent_debt_data_key = parent_debt_key(market_id, user.clone(), redeemed_outcome);
        let parent_debt = read_i128(&env, &parent_debt_data_key);
        let remaining_claim = checked_sub(gross_payout, child_repayment)?;
        let parent_repayment = if parent_debt > remaining_claim {
            remaining_claim
        } else {
            parent_debt
        };
        if parent_debt > 0 {
            let unpaid_parent_debt = checked_sub(parent_debt, parent_repayment)?;
            write_i128(&env, &parent_debt_data_key, unpaid_parent_debt);
            if parent_repayment > 0 {
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_outcome_key(market_id, user.clone(), redeemed_outcome),
                    parent_repayment,
                )?;
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_key(market_id, user.clone()),
                    parent_repayment,
                )?;
                let mut this_accounting = read_accounting(&env, market_id);
                this_accounting.collateral_backing =
                    checked_add(this_accounting.collateral_backing, parent_repayment)?;
                this_accounting.refundable =
                    checked_add(this_accounting.refundable, parent_repayment)?;
                this_accounting.child_collateral_repaid =
                    checked_add(this_accounting.child_collateral_repaid, parent_repayment)?;
                write_accounting(&env, market_id, &this_accounting);
            }
            // A total loss (gross_payout == 0) leaves nothing to withhold from —
            // whatever's still owed becomes an unrecoverable default, handled
            // via the insurance-then-shortfall path instead of a free write-off.
            if gross_payout == 0 && unpaid_parent_debt > 0 {
                resolve_parent_default(
                    &env,
                    market_id,
                    user.clone(),
                    redeemed_outcome,
                    unpaid_parent_debt,
                )?;
            }
        }

        // Now cap whatever's left of the claim (after debt already withheld)
        // against this market's own backing — which, if debt was just repaid
        // above, is now restored and reflects the healthy state, not the
        // temporarily-depleted one. Any cap that still applies here reflects
        // a REAL, separate, already-crystallized shortfall from elsewhere.
        let remaining_after_debt = checked_sub(remaining_claim, parent_repayment)?;
        let payout = capped_payout(&env, market_id, remaining_after_debt)?;

        DikeTokensClient::new(&env, &tokens).burn_for_redeem(
            &user,
            &market_id,
            &redeemed_outcome,
            &amount,
        );

        if gross_payout > 0 {
            // Deposit/stake bookkeeping tracks the user's own claim being
            // retired in full (tokens are burned in full above) — separate
            // from `payout`, which is how much *real cash* actually left the
            // building for this redemption.
            let deposit_key = user_deposit_key(market_id, user.clone());
            let stake_key = root_stake_key(market_id, user.clone(), redeemed_outcome);
            let _ = saturating_sub_i128(&env, &deposit_key, gross_payout)?;
            let _ = saturating_sub_i128(&env, &stake_key, gross_payout)?;
        }
        if payout > 0 {
            let mut accounting = read_accounting(&env, market_id);
            accounting.redeemed = checked_add(accounting.redeemed, payout)?;
            accounting.refundable = checked_sub(accounting.refundable, payout)?;
            write_accounting(&env, market_id, &accounting);
            transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
        }
        // gross_payout == 0 && parent_debt > 0 is already fully handled above via
        // resolve_parent_default (insurance draw + shortfall bookkeeping on the
        // affected children), so there's nothing further to do here.

        Redeemed {
            market_id,
            user,
            outcome: redeemed_outcome,
            payout,
        }
        .publish(&env);
        Ok(payout)
    }

    pub fn redeem_cancelled(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        redeemed_outcome: Outcome,
        amount: i128,
    ) -> Result<i128, DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let market = market_data(&env, market_id)?;
        if market.status != MarketStatus::Cancelled || token != market.collateral {
            return Err(DikeError::InvalidStatus);
        }
        let tokens = read_role(&env, symbol_short!("tokens"))?;
        DikeTokensClient::new(&env, &tokens).burn_for_redeem(
            &user,
            &market_id,
            &redeemed_outcome,
            &amount,
        );
        // Fold in any prior dust carry so odd-stroop remainders never vanish.
        let carry_key = DataKey::InvalidDustCarry(market_id);
        let carry = read_i128(&env, &carry_key);
        let effective = checked_add(carry, amount)?;
        let gross_payout = invalid_refund(effective)?;
        let new_carry = checked_sub(effective, checked_add(gross_payout, gross_payout)?)?;
        write_i128(&env, &carry_key, new_carry);

        // Same ordering as redeem_resolved and for the same reason: withhold
        // against outstanding parent_debt FIRST, against the full theoretical
        // refund — restoring this market's own backing by the withheld amount
        // (self-contained, same market crediting itself back) — THEN cap
        // whatever's left against real availability. A cancelled market that
        // was still backing live child credit can't be cashed out for free
        // (previously this was skipped entirely: a user could open child
        // credit, get the parent cancelled, and walk with the full refund
        // while the child debt stayed marked healthy against collateral that
        // had already been paid out).
        let parent_debt_data_key = parent_debt_key(market_id, user.clone(), redeemed_outcome);
        let parent_debt = read_i128(&env, &parent_debt_data_key);
        let withheld = if parent_debt > gross_payout {
            gross_payout
        } else {
            parent_debt
        };
        if parent_debt > 0 {
            let unpaid_parent_debt = checked_sub(parent_debt, withheld)?;
            write_i128(&env, &parent_debt_data_key, unpaid_parent_debt);
            if withheld > 0 {
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_outcome_key(market_id, user.clone(), redeemed_outcome),
                    withheld,
                )?;
                let _ =
                    saturating_sub_i128(&env, &child_used_key(market_id, user.clone()), withheld)?;
                let mut this_accounting = read_accounting(&env, market_id);
                this_accounting.collateral_backing =
                    checked_add(this_accounting.collateral_backing, withheld)?;
                this_accounting.refundable = checked_add(this_accounting.refundable, withheld)?;
                this_accounting.child_collateral_repaid =
                    checked_add(this_accounting.child_collateral_repaid, withheld)?;
                write_accounting(&env, market_id, &this_accounting);
            }
            if gross_payout == 0 && unpaid_parent_debt > 0 {
                resolve_parent_default(
                    &env,
                    market_id,
                    user.clone(),
                    redeemed_outcome,
                    unpaid_parent_debt,
                )?;
            }
        }

        let remaining_after_debt = checked_sub(gross_payout, withheld)?;
        let payout = capped_payout(&env, market_id, remaining_after_debt)?;

        if payout > 0 {
            let mut accounting = read_accounting(&env, market_id);
            accounting.redeemed = checked_add(accounting.redeemed, payout)?;
            accounting.refundable = checked_sub(accounting.refundable, payout)?;
            write_accounting(&env, market_id, &accounting);
            transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
        }
        Redeemed {
            market_id,
            user,
            outcome: redeemed_outcome,
            payout,
        }
        .publish(&env);
        Ok(payout)
    }

    pub fn assert_position_transfer_allowed(
        env: Env,
        from: Address,
        market_id: u64,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("tokens"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        if read_i128(&env, &child_debt_key(market_id, from.clone())) > 0
            || read_i128(&env, &parent_debt_key(market_id, from, outcome)) > 0
        {
            return Err(DikeError::EncumberedPosition);
        }
        Ok(())
    }

    pub fn lock_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        market_id: u64,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, market_id)?;
        let bond_key = DataKey::Bond(request_id, user.clone(), is_dispute);
        let bond_token_key = DataKey::BondToken(request_id, user.clone(), is_dispute);
        let bond_market_key = DataKey::BondMarket(request_id, user.clone(), is_dispute);
        if env.storage().persistent().has(&bond_key) {
            return Err(DikeError::InvalidInput);
        }
        transfer_token(&env, &token, &user, &env.current_contract_address(), amount);
        env.storage().persistent().set(&bond_key, &amount);
        env.storage().persistent().set(&bond_token_key, &token);
        env.storage().persistent().set(&bond_market_key, &market_id);
        env.storage()
            .persistent()
            .extend_ttl(&bond_key, MIN_TTL, EXTEND_TTL);
        env.storage()
            .persistent()
            .extend_ttl(&bond_token_key, MIN_TTL, EXTEND_TTL);
        env.storage()
            .persistent()
            .extend_ttl(&bond_market_key, MIN_TTL, EXTEND_TTL);
        let mut accounting = read_accounting(&env, market_id);
        if is_dispute {
            accounting.dispute_bonds = checked_add(accounting.dispute_bonds, amount)?;
        } else {
            accounting.proposal_bonds = checked_add(accounting.proposal_bonds, amount)?;
        }
        write_accounting(&env, market_id, &accounting);
        BondLocked {
            request_id,
            user,
            amount,
            is_dispute,
        }
        .publish(&env);
        Ok(())
    }
    pub fn release_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let bond_key = DataKey::Bond(request_id, user.clone(), is_dispute);
        let bond_token_key = DataKey::BondToken(request_id, user.clone(), is_dispute);
        let bond_market_key = DataKey::BondMarket(request_id, user.clone(), is_dispute);
        let locked_token: Address = env
            .storage()
            .persistent()
            .get(&bond_token_key)
            .ok_or(DikeError::InsufficientBalance)?;
        if locked_token != token {
            return Err(DikeError::UnsupportedCollateral);
        }
        let locked: i128 = env
            .storage()
            .persistent()
            .get(&bond_key)
            .ok_or(DikeError::InsufficientBalance)?;
        if locked < amount {
            return Err(DikeError::InsufficientBalance);
        }
        let market_id: u64 = env
            .storage()
            .persistent()
            .get(&bond_market_key)
            .ok_or(DikeError::MarketNotFound)?;
        env.storage()
            .persistent()
            .set(&bond_key, &checked_sub(locked, amount)?);
        let mut accounting = read_accounting(&env, market_id);
        if is_dispute {
            accounting.dispute_bonds = checked_sub(accounting.dispute_bonds, amount)?;
        } else {
            accounting.proposal_bonds = checked_sub(accounting.proposal_bonds, amount)?;
        }
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &user, amount);
        BondReleased {
            request_id,
            user,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn slash_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        amount: i128,
        is_dispute: bool,
        recipient: Address,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let bond_key = DataKey::Bond(request_id, user.clone(), is_dispute);
        let bond_token_key = DataKey::BondToken(request_id, user.clone(), is_dispute);
        let bond_market_key = DataKey::BondMarket(request_id, user, is_dispute);
        let locked_token: Address = env
            .storage()
            .persistent()
            .get(&bond_token_key)
            .ok_or(DikeError::InsufficientBalance)?;
        if locked_token != token {
            return Err(DikeError::UnsupportedCollateral);
        }
        let locked: i128 = env
            .storage()
            .persistent()
            .get(&bond_key)
            .ok_or(DikeError::InsufficientBalance)?;
        if locked < amount {
            return Err(DikeError::InsufficientBalance);
        }
        let market_id: u64 = env
            .storage()
            .persistent()
            .get(&bond_market_key)
            .ok_or(DikeError::MarketNotFound)?;
        env.storage()
            .persistent()
            .set(&bond_key, &checked_sub(locked, amount)?);
        let mut accounting = read_accounting(&env, market_id);
        if is_dispute {
            accounting.dispute_bonds = checked_sub(accounting.dispute_bonds, amount)?;
        } else {
            accounting.proposal_bonds = checked_sub(accounting.proposal_bonds, amount)?;
        }
        write_accounting(&env, market_id, &accounting);
        transfer_token(
            &env,
            &token,
            &env.current_contract_address(),
            &recipient,
            amount,
        );
        Ok(())
    }

    pub fn collect_fee(
        env: Env,
        market_id: u64,
        lp_fee: i128,
        protocol_fee: i128,
        cod_fee: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if lp_fee < 0 || protocol_fee < 0 || cod_fee < 0 {
            return Err(DikeError::InvalidAmount);
        }
        let total_fee = checked_add(checked_add(lp_fee, protocol_fee)?, cod_fee)?;
        let mut accounting = read_accounting(&env, market_id);
        if accounting.collateral_backing < total_fee || accounting.refundable < total_fee {
            return Err(DikeError::InsufficientCollateral);
        }
        accounting.collateral_backing = checked_sub(accounting.collateral_backing, total_fee)?;
        accounting.refundable = checked_sub(accounting.refundable, total_fee)?;
        accounting.lp_fees = checked_add(accounting.lp_fees, lp_fee)?;
        accounting.protocol_fees = checked_add(accounting.protocol_fees, protocol_fee)?;
        accounting.cod_fees = checked_add(accounting.cod_fees, cod_fee)?;
        write_accounting(&env, market_id, &accounting);
        FeesCollected {
            market_id,
            lp_fee,
            protocol_fee,
            cod_fee,
        }
        .publish(&env);
        Ok(())
    }

    pub fn claim_lp_fees(
        env: Env,
        token: Address,
        market_id: u64,
        lp: Address,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, market_id)?;
        let mut accounting = read_accounting(&env, market_id);
        if accounting.lp_fees < amount {
            return Err(DikeError::InsufficientBalance);
        }
        accounting.lp_fees = checked_sub(accounting.lp_fees, amount)?;
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &lp, amount);
        Ok(())
    }

    pub fn sweep_protocol_fees(
        env: Env,
        token: Address,
        market_id: u64,
    ) -> Result<i128, DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        require_market_collateral(&env, &token, market_id)?;
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(DikeError::NotInitialized)?;
        let mut accounting = read_accounting(&env, market_id);
        let amount = checked_add(accounting.protocol_fees, accounting.cod_fees)?;
        accounting.protocol_fees = 0;
        accounting.cod_fees = 0;
        write_accounting(&env, market_id, &accounting);
        if amount > 0 {
            // Hold back a slice into the insurance reserve instead of
            // sweeping it all to treasury — it's already real money sitting
            // in the vault's balance, just deliberately not paid out, so
            // there's a real backstop for `resolve_parent_default` to draw on.
            let reserve_cut = bps(amount, INSURANCE_RESERVE_BPS)?;
            let treasury_amount = checked_sub(amount, reserve_cut)?;
            if reserve_cut > 0 {
                write_insurance_reserve(
                    &env,
                    checked_add(read_insurance_reserve(&env), reserve_cut)?,
                );
            }
            if treasury_amount > 0 {
                transfer_token(
                    &env,
                    &token,
                    &env.current_contract_address(),
                    &treasury,
                    treasury_amount,
                );
            }
        }
        Ok(amount)
    }

    pub fn insurance_reserve(env: Env) -> i128 {
        read_insurance_reserve(&env)
    }

    pub fn accounting(env: Env, market_id: u64) -> VaultAccounting {
        read_accounting(&env, market_id)
    }

    pub fn user_deposit(env: Env, market_id: u64, user: Address) -> i128 {
        read_i128(&env, &user_deposit_key(market_id, user))
    }

    pub fn root_stake(env: Env, market_id: u64, user: Address, outcome: Outcome) -> i128 {
        read_i128(&env, &root_stake_key(market_id, user, outcome))
    }

    pub fn child_collateral_used(env: Env, parent_market_id: u64, user: Address) -> i128 {
        read_i128(&env, &child_used_key(parent_market_id, user))
    }

    pub fn child_used_for_outcome(
        env: Env,
        parent_market_id: u64,
        user: Address,
        outcome: Outcome,
    ) -> i128 {
        read_i128(
            &env,
            &child_used_outcome_key(parent_market_id, user, outcome),
        )
    }

    pub fn child_collateral_available(
        env: Env,
        parent_market_id: u64,
        user: Address,
    ) -> Result<i128, DikeError> {
        let deposit = read_i128(&env, &user_deposit_key(parent_market_id, user.clone()));
        let limit = collateral_limit(deposit, DEFAULT_CHILD_COLLATERAL_BPS)?;
        let used = read_i128(&env, &child_used_key(parent_market_id, user));
        checked_sub(limit, used)
    }

    pub fn child_avail_for_outcome(
        env: Env,
        parent_market_id: u64,
        user: Address,
        outcome: Outcome,
    ) -> Result<i128, DikeError> {
        let stake = read_i128(
            &env,
            &root_stake_key(parent_market_id, user.clone(), outcome),
        );
        let limit = collateral_limit(stake, DEFAULT_CHILD_COLLATERAL_BPS)?;
        let used = read_i128(
            &env,
            &child_used_outcome_key(parent_market_id, user, outcome),
        );
        checked_sub(limit, used)
    }

    pub fn child_parent(env: Env, child_market_id: u64, user: Address) -> u64 {
        read_parent(&env, child_market_id, user)
    }

    /// Every child market a given parent+user+outcome stake has drawn credit
    /// into. Lets keepers discover liquidation targets on-chain instead of
    /// relying on off-chain event indexing (and an incomplete keeper-supplied
    /// list stranding debt).
    pub fn child_markets_of(
        env: Env,
        parent_market_id: u64,
        user: Address,
        outcome: Outcome,
    ) -> Vec<u64> {
        read_child_markets_of(&env, parent_market_id, user, outcome)
    }

    pub fn child_parent_outcome(env: Env, child_market_id: u64, user: Address) -> Outcome {
        read_parent_outcome(&env, child_market_id, user)
    }

    pub fn child_collateral_loan(
        env: Env,
        parent_market_id: u64,
        child_market_id: u64,
        user: Address,
    ) -> i128 {
        read_i128(
            &env,
            &child_loan_key(parent_market_id, child_market_id, user),
        )
    }

    pub fn child_loan_for_outcome(
        env: Env,
        parent_market_id: u64,
        parent_outcome: Outcome,
        child_market_id: u64,
        child_outcome: Outcome,
        user: Address,
    ) -> i128 {
        read_i128(
            &env,
            &child_loan_outcome_key(
                parent_market_id,
                parent_outcome,
                child_market_id,
                child_outcome,
                user,
            ),
        )
    }

    pub fn child_debt(env: Env, child_market_id: u64, user: Address) -> i128 {
        read_i128(&env, &child_debt_key(child_market_id, user))
    }

    pub fn parent_debt(env: Env, parent_market_id: u64, user: Address, outcome: Outcome) -> i128 {
        read_i128(&env, &parent_debt_key(parent_market_id, user, outcome))
    }

    pub fn redeemed(env: Env, market_id: u64, user: Address, outcome: Outcome) -> i128 {
        let key = DataKey::Redeemed(market_id, user, outcome);
        if !env.storage().persistent().has(&key) {
            return 0;
        }
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}

mod test;

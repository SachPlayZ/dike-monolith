#![no_std]

use dike_math::{checked_add, checked_sub, collateral_limit, invalid_refund};
use dike_types::{
    DikeError, MarketData, MarketStatus, Outcome, VaultAccounting, DEFAULT_CHILD_COLLATERAL_BPS,
};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short,
    token::Client as TokenClient, Address, Env, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

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
    Bond(u64, Address, bool),
    BondToken(u64, Address, bool),
    BondMarket(u64, Address, bool),
    Redeemed(u64, Address, Outcome),
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

#[contractevent(topics = ["release"], data_format = "single-value")]
#[derive(Clone)]
pub struct MergeRelease {
    #[topic]
    pub market_id: u64,
    #[topic]
    pub user: Address,
    pub amount: i128,
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

fn reduce_user_deposit(
    env: &Env,
    market_id: u64,
    user: Address,
    amount: i128,
) -> Result<(), DikeError> {
    let deposit_key = user_deposit_key(market_id, user.clone());
    let current = read_i128(env, &deposit_key);
    let used = read_i128(env, &child_used_key(market_id, user));
    if current == 0 {
        if used > 0 {
            return Err(DikeError::InsufficientCollateral);
        }
        return Ok(());
    }
    if current < amount {
        return Err(DikeError::InsufficientBalance);
    }
    let next = checked_sub(current, amount)?;
    let next_limit = collateral_limit(next, DEFAULT_CHILD_COLLATERAL_BPS)?;
    if used > next_limit {
        return Err(DikeError::ChildCollateralLimitExceeded);
    }
    write_i128(env, &deposit_key, next);
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

    pub fn release_on_merge(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("tokens"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        require_market_collateral(&env, &token, market_id)?;
        let mut accounting = read_accounting(&env, market_id);
        if accounting.collateral_backing < amount || accounting.refundable < amount {
            return Err(DikeError::InsufficientCollateral);
        }
        reduce_user_deposit(&env, market_id, user.clone(), amount)?;
        accounting.collateral_backing = checked_sub(accounting.collateral_backing, amount)?;
        accounting.refundable = checked_sub(accounting.refundable, amount)?;
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &user, amount);
        MergeRelease {
            market_id,
            user,
            amount,
        }
        .publish(&env);
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

        let mut parent_accounting = read_accounting(&env, parent_market_id);
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

    pub fn repay_child_collateral(
        env: Env,
        _token: Address,
        _user: Address,
        _parent_market_id: u64,
        _child_market_id: u64,
        _amount: i128,
    ) -> Result<(), DikeError> {
        let _ = env;
        Err(DikeError::Unauthorized)
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
        let stake_key = root_stake_key(market_id, user.clone(), outcome);
        let deposit_key = user_deposit_key(market_id, user.clone());
        let _ = saturating_sub_i128(&env, &stake_key, tokens_sold)?;
        let _ = saturating_sub_i128(&env, &deposit_key, payout)?;
        let mut accounting = read_accounting(&env, market_id);
        if accounting.collateral_backing < payout || accounting.refundable < payout {
            return Err(DikeError::InsufficientCollateral);
        }
        accounting.collateral_backing = checked_sub(accounting.collateral_backing, payout)?;
        accounting.refundable = checked_sub(accounting.refundable, payout)?;
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
        Ok(())
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
        DikeTokensClient::new(&env, &tokens).burn_for_redeem(
            &user,
            &market_id,
            &redeemed_outcome,
            &amount,
        );

        let gross_payout = match final_outcome {
            Outcome::Invalid => invalid_refund(amount)?,
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

        let debt_key = child_debt_key(market_id, user.clone());
        let raw_child_debt = read_i128(&env, &debt_key);
        let parent_market_id = read_parent(&env, market_id, user.clone());
        let parent_outcome = read_parent_outcome(&env, market_id, user.clone());
        let effective_child_debt = if raw_child_debt > 0 && parent_market_id != 0 {
            let upstream_key = parent_debt_key(parent_market_id, user.clone(), parent_outcome);
            let upstream_debt = read_i128(&env, &upstream_key);
            if upstream_debt == 0 {
                write_i128(&env, &debt_key, 0);
                0
            } else if upstream_debt < raw_child_debt {
                upstream_debt
            } else {
                raw_child_debt
            }
        } else {
            raw_child_debt
        };
        let parent_debt_data_key = parent_debt_key(market_id, user.clone(), redeemed_outcome);
        let parent_debt = read_i128(&env, &parent_debt_data_key);
        let debt = checked_add(effective_child_debt, parent_debt)?;
        let repayment = if gross_payout > debt {
            debt
        } else {
            gross_payout
        };
        let payout = checked_sub(gross_payout, repayment)?;
        if effective_child_debt > 0 {
            let child_repayment = if repayment > effective_child_debt {
                effective_child_debt
            } else {
                repayment
            };
            if child_repayment > 0 {
                write_i128(
                    &env,
                    &debt_key,
                    checked_sub(raw_child_debt, child_repayment)?,
                );
                if parent_market_id != 0 {
                    let upstream_key =
                        parent_debt_key(parent_market_id, user.clone(), parent_outcome);
                    let upstream_debt = read_i128(&env, &upstream_key);
                    write_i128(
                        &env,
                        &upstream_key,
                        checked_sub(upstream_debt, child_repayment)?,
                    );
                    let _ = saturating_sub_i128(
                        &env,
                        &child_used_outcome_key(parent_market_id, user.clone(), parent_outcome),
                        child_repayment,
                    )?;
                    let _ = saturating_sub_i128(
                        &env,
                        &child_used_key(parent_market_id, user.clone()),
                        child_repayment,
                    )?;
                }
            }
        }
        if parent_debt > 0 {
            let remaining_repayment = if repayment > effective_child_debt {
                checked_sub(repayment, effective_child_debt)?
            } else {
                0
            };
            let unpaid_parent_debt = checked_sub(parent_debt, remaining_repayment)?;
            write_i128(&env, &parent_debt_data_key, unpaid_parent_debt);
            if remaining_repayment > 0 {
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_outcome_key(market_id, user.clone(), redeemed_outcome),
                    remaining_repayment,
                )?;
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_key(market_id, user.clone()),
                    remaining_repayment,
                )?;
            }
            if gross_payout == 0 && unpaid_parent_debt > 0 {
                write_i128(&env, &parent_debt_data_key, 0);
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_outcome_key(market_id, user.clone(), redeemed_outcome),
                    unpaid_parent_debt,
                )?;
                let _ = saturating_sub_i128(
                    &env,
                    &child_used_key(market_id, user.clone()),
                    unpaid_parent_debt,
                )?;
            }
        }

        if gross_payout > 0 {
            let mut accounting = read_accounting(&env, market_id);
            let max_remaining = checked_sub(accounting.collateral_backing, accounting.redeemed)?;
            if gross_payout > max_remaining {
                return Err(DikeError::InsufficientCollateral);
            }
            let deposit_key = user_deposit_key(market_id, user.clone());
            let stake_key = root_stake_key(market_id, user.clone(), redeemed_outcome);
            let _ = saturating_sub_i128(&env, &deposit_key, gross_payout)?;
            let _ = saturating_sub_i128(&env, &stake_key, gross_payout)?;
            accounting.redeemed = checked_add(accounting.redeemed, gross_payout)?;
            accounting.refundable = checked_sub(accounting.refundable, gross_payout)?;
            write_accounting(&env, market_id, &accounting);
            if payout > 0 {
                transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
            }
        } else if parent_debt > 0 {
            let mut accounting = read_accounting(&env, market_id);
            accounting.child_collateral_defaulted =
                checked_add(accounting.child_collateral_defaulted, parent_debt)?;
            write_accounting(&env, market_id, &accounting);
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
        let payout = invalid_refund(amount)?;
        if payout > 0 {
            let mut accounting = read_accounting(&env, market_id);
            let max_remaining = checked_sub(accounting.collateral_backing, accounting.redeemed)?;
            if payout > max_remaining {
                return Err(DikeError::InsufficientCollateral);
            }
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
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &treasury,
                amount,
            );
        }
        Ok(amount)
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

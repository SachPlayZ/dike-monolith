#![no_std]
#![allow(clippy::too_many_arguments)]

use dike_math::{
    average_price_bps, bps, checked_add, checked_sub, proportional, quote_buy_complete_set,
    quote_sell, split_fee,
};
use dike_types::{DikeError, FeeConfig, MarketData, MarketStatus, Outcome, PoolData, TradeQuote};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short, Address,
    Env, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Vault,
    Tokens,
    Collateral,
    Registry,
    Pool(u64),
    PoolFee(u64),
    LpBalance(u64, Address),
    NextPoolId,
    Paused,
}

#[contractclient(name = "DikeVaultClient")]
pub trait DikeVault {
    fn deposit_for_market(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        amount: i128,
    ) -> Result<(), DikeError>;

    fn record_cash_stake(
        env: Env,
        user: Address,
        market_id: u64,
        outcome: Outcome,
        collateral_in: i128,
        tokens_out: i128,
    ) -> Result<(), DikeError>;

    fn open_child_credit_for_trade(
        env: Env,
        user: Address,
        parent_market_id: u64,
        parent_outcome: Outcome,
        child_market_id: u64,
        child_outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError>;

    fn release_trade_payout(
        env: Env,
        token: Address,
        user: Address,
        market_id: u64,
        outcome: Outcome,
        tokens_sold: i128,
        payout: i128,
    ) -> Result<(), DikeError>;

    fn collect_fee(
        env: Env,
        market_id: u64,
        lp_fee: i128,
        protocol_fee: i128,
        cod_fee: i128,
    ) -> Result<(), DikeError>;
}

#[contractclient(name = "DikeRegistryClient")]
pub trait DikeRegistry {
    fn is_tradeable(env: Env, market_id: u64) -> Result<bool, DikeError>;
    fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError>;
}

#[contractclient(name = "DikeTokensClient")]
pub trait DikeTokens {
    fn mint_complete_set(
        env: Env,
        to: Address,
        market_id: u64,
        amount: i128,
    ) -> Result<(), DikeError>;

    fn transfer_position(
        env: Env,
        from: Address,
        to: Address,
        market_id: u64,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError>;

    fn merge_positions(
        env: Env,
        user: Address,
        market_id: u64,
        amount: i128,
    ) -> Result<(), DikeError>;
}

#[contractevent(topics = ["role"], data_format = "single-value")]
#[derive(Clone)]
pub struct RoleSet {
    #[topic]
    pub role: Symbol,
    pub module: Address,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent(topics = ["pool"], data_format = "single-value")]
#[derive(Clone)]
pub struct PoolCreated {
    #[topic]
    pub market_id: u64,
    pub pool_id: u64,
}

#[contractevent(topics = ["seed"], data_format = "single-value")]
#[derive(Clone)]
pub struct LiquiditySeeded {
    #[topic]
    pub pool_id: u64,
    #[topic]
    pub lp: Address,
    pub amount: i128,
}

#[contractevent(topics = ["lp_add"], data_format = "vec")]
#[derive(Clone)]
pub struct LiquidityAdded {
    #[topic]
    pub pool_id: u64,
    #[topic]
    pub lp: Address,
    pub amount: i128,
    pub shares: i128,
}

#[contractevent(topics = ["lp_rm"], data_format = "vec")]
#[derive(Clone)]
pub struct LiquidityRemoved {
    #[topic]
    pub pool_id: u64,
    #[topic]
    pub lp: Address,
    pub shares: i128,
    pub yes_out: i128,
    pub no_out: i128,
}

#[contractevent(topics = ["buy"], data_format = "vec")]
#[derive(Clone)]
pub struct BuyExecuted {
    #[topic]
    pub pool_id: u64,
    #[topic]
    pub trader: Address,
    pub yes: bool,
    pub amount_in: i128,
    pub amount_out: i128,
}

#[contractevent(topics = ["sell"], data_format = "vec")]
#[derive(Clone)]
pub struct SellExecuted {
    #[topic]
    pub pool_id: u64,
    #[topic]
    pub trader: Address,
    pub yes: bool,
    pub amount_in: i128,
    pub amount_out: i128,
}

#[contract]
pub struct DikeAMM;

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);
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

fn read_module(env: &Env, key: DataKey) -> Result<Address, DikeError> {
    env.storage()
        .instance()
        .get(&key)
        .ok_or(DikeError::NotInitialized)
}

fn modules(env: &Env) -> Result<(Address, Address, Address, Address), DikeError> {
    Ok((
        read_module(env, DataKey::Vault)?,
        read_module(env, DataKey::Tokens)?,
        read_module(env, DataKey::Collateral)?,
        read_module(env, DataKey::Registry)?,
    ))
}

fn read_pool(env: &Env, pool_id: u64) -> Result<PoolData, DikeError> {
    let key = DataKey::Pool(pool_id);
    if !env.storage().persistent().has(&key) {
        return Err(DikeError::PoolNotFound);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(DikeError::PoolNotFound)
}

fn write_pool(env: &Env, pool: &PoolData) {
    let key = DataKey::Pool(pool.id);
    env.storage().persistent().set(&key, pool);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn read_fee(env: &Env, pool_id: u64) -> FeeConfig {
    let key = DataKey::PoolFee(pool_id);
    if !env.storage().persistent().has(&key) {
        return FeeConfig::default();
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(&key).unwrap_or_default()
}

fn write_fee(env: &Env, pool_id: u64, fee: &FeeConfig) {
    let key = DataKey::PoolFee(pool_id);
    env.storage().persistent().set(&key, fee);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn validate_fee_config(config: &FeeConfig) -> Result<(), DikeError> {
    let share_total = config.lp_fee_share_bps as u64
        + config.treasury_fee_share_bps as u64
        + config.cod_fee_share_bps as u64;
    if share_total != 10_000 || config.trading_fee_bps > 1_000 {
        return Err(DikeError::InvalidInput);
    }
    if config.proposal_reward < 0
        || config.dispute_reward < 0
        || config.council_reward < 0
        || config.creation_fee < 0
    {
        return Err(DikeError::InvalidAmount);
    }
    Ok(())
}

fn require_live(env: &Env, pool: &PoolData, deadline: u64) -> Result<(), DikeError> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused || !pool.live {
        return Err(DikeError::InvalidStatus);
    }
    if env.ledger().timestamp() > deadline {
        return Err(DikeError::DeadlineExpired);
    }
    let registry = read_module(env, DataKey::Registry)?;
    if !DikeRegistryClient::new(env, &registry).is_tradeable(&pool.market_id) {
        return Err(DikeError::InvalidStatus);
    }
    Ok(())
}

fn require_market_tradeable(env: &Env, market_id: u64) -> Result<(), DikeError> {
    let registry = read_module(env, DataKey::Registry)?;
    if !DikeRegistryClient::new(env, &registry).is_tradeable(&market_id) {
        return Err(DikeError::InvalidStatus);
    }
    Ok(())
}

fn require_market_liquidity_removable(env: &Env, market_id: u64) -> Result<(), DikeError> {
    let registry = read_module(env, DataKey::Registry)?;
    let market = DikeRegistryClient::new(env, &registry).get_market(&market_id);
    match market.status {
        MarketStatus::Live => {
            if market.has_final_outcome || env.ledger().timestamp() >= market.expiry {
                return Err(DikeError::InvalidStatus);
            }
        }
        MarketStatus::Cancelled => {
            if market.has_final_outcome {
                return Err(DikeError::InvalidStatus);
            }
        }
        _ => return Err(DikeError::InvalidStatus),
    }
    Ok(())
}

fn require_market_seedable(env: &Env, market_id: u64) -> Result<(), DikeError> {
    let registry = read_module(env, DataKey::Registry)?;
    let market = DikeRegistryClient::new(env, &registry).get_market(&market_id);
    let expired = env.ledger().timestamp() >= market.expiry;
    if expired
        || (market.status != MarketStatus::Created && market.status != MarketStatus::Live)
        || market.has_final_outcome
    {
        return Err(DikeError::InvalidStatus);
    }
    Ok(())
}

fn accrue_fees(
    pool: &mut PoolData,
    fee_config: &FeeConfig,
    total_fee: i128,
) -> Result<(i128, i128, i128), DikeError> {
    let lp_fee = bps(total_fee, fee_config.lp_fee_share_bps)?;
    let treasury_fee = bps(total_fee, fee_config.treasury_fee_share_bps)?;
    let cod_fee = checked_sub(checked_sub(total_fee, lp_fee)?, treasury_fee)?;
    pool.accumulated_lp_fees = checked_add(pool.accumulated_lp_fees, lp_fee)?;
    pool.accumulated_protocol_fees = checked_add(pool.accumulated_protocol_fees, treasury_fee)?;
    pool.accumulated_cod_fees = checked_add(pool.accumulated_cod_fees, cod_fee)?;
    Ok((lp_fee, treasury_fee, cod_fee))
}

fn lp_key(pool_id: u64, owner: Address) -> DataKey {
    DataKey::LpBalance(pool_id, owner)
}

fn read_lp(env: &Env, pool_id: u64, owner: Address) -> i128 {
    let key = lp_key(pool_id, owner);
    if !env.storage().persistent().has(&key) {
        return 0;
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(&key).unwrap_or(0)
}

fn write_lp(env: &Env, pool_id: u64, owner: Address, amount: i128) {
    let key = lp_key(pool_id, owner);
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn quote_buy_side(
    env: Env,
    pool_id: u64,
    amount_in: i128,
    yes: bool,
) -> Result<TradeQuote, DikeError> {
    if amount_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let pool = read_pool(&env, pool_id)?;
    let fee_config = read_fee(&env, pool_id);
    let (fee, net_in) = split_fee(amount_in, fee_config.trading_fee_bps)?;
    let amount_out = if yes {
        quote_buy_complete_set(pool.yes_reserve, pool.no_reserve, net_in)?
    } else {
        quote_buy_complete_set(pool.no_reserve, pool.yes_reserve, net_in)?
    };
    Ok(TradeQuote {
        amount_in,
        fee,
        net_in,
        amount_out,
        average_price_bps: average_price_bps(amount_in, amount_out)?,
    })
}

fn buy(
    env: Env,
    trader: Address,
    pool_id: u64,
    amount_in: i128,
    min_out: i128,
    deadline: u64,
    yes: bool,
    parent: Option<(u64, Outcome)>,
) -> Result<i128, DikeError> {
    trader.require_auth();
    if amount_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let mut pool = read_pool(&env, pool_id)?;
    require_live(&env, &pool, deadline)?;
    let (vault, tokens, collateral, _) = modules(&env)?;
    let fee_config = read_fee(&env, pool_id);
    let (fee, net_in) = split_fee(amount_in, fee_config.trading_fee_bps)?;
    let out = if yes {
        quote_buy_complete_set(pool.yes_reserve, pool.no_reserve, net_in)?
    } else {
        quote_buy_complete_set(pool.no_reserve, pool.yes_reserve, net_in)?
    };
    if out < min_out {
        return Err(DikeError::SlippageExceeded);
    }
    let outcome = if yes { Outcome::Yes } else { Outcome::No };
    let vault_client = DikeVaultClient::new(&env, &vault);
    if let Some((parent_market_id, parent_outcome)) = parent {
        vault_client.open_child_credit_for_trade(
            &trader,
            &parent_market_id,
            &parent_outcome,
            &pool.market_id,
            &outcome,
            &amount_in,
        );
    } else {
        vault_client.deposit_for_market(&collateral, &trader, &pool.market_id, &amount_in);
        vault_client.record_cash_stake(&trader, &pool.market_id, &outcome, &amount_in, &out);
    }
    DikeTokensClient::new(&env, &tokens).mint_complete_set(
        &env.current_contract_address(),
        &pool.market_id,
        &net_in,
    );
    let (lp_fee, treasury_fee, cod_fee) = accrue_fees(&mut pool, &fee_config, fee)?;
    vault_client.collect_fee(&pool.market_id, &lp_fee, &treasury_fee, &cod_fee);
    if yes {
        pool.yes_reserve = checked_sub(checked_add(pool.yes_reserve, net_in)?, out)?;
        pool.no_reserve = checked_add(pool.no_reserve, net_in)?;
    } else {
        pool.no_reserve = checked_sub(checked_add(pool.no_reserve, net_in)?, out)?;
        pool.yes_reserve = checked_add(pool.yes_reserve, net_in)?;
    }
    DikeTokensClient::new(&env, &tokens).transfer_position(
        &env.current_contract_address(),
        &trader,
        &pool.market_id,
        &outcome,
        &out,
    );
    write_pool(&env, &pool);
    BuyExecuted {
        pool_id,
        trader,
        yes,
        amount_in,
        amount_out: out,
    }
    .publish(&env);
    Ok(out)
}

fn sell(
    env: Env,
    trader: Address,
    pool_id: u64,
    amount_in: i128,
    min_out: i128,
    deadline: u64,
    yes: bool,
) -> Result<i128, DikeError> {
    trader.require_auth();
    if amount_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let mut pool = read_pool(&env, pool_id)?;
    require_live(&env, &pool, deadline)?;
    let (vault, tokens, collateral, _) = modules(&env)?;
    let fee_config = read_fee(&env, pool_id);
    let gross_out = if yes {
        quote_sell(pool.yes_reserve, pool.no_reserve, amount_in)?
    } else {
        quote_sell(pool.no_reserve, pool.yes_reserve, amount_in)?
    };
    let (fee, net_out) = split_fee(gross_out, fee_config.trading_fee_bps)?;
    if net_out < min_out {
        return Err(DikeError::SlippageExceeded);
    }
    let (lp_fee, treasury_fee, cod_fee) = accrue_fees(&mut pool, &fee_config, fee)?;
    let outcome = if yes { Outcome::Yes } else { Outcome::No };
    let token_client = DikeTokensClient::new(&env, &tokens);
    token_client.transfer_position(
        &trader,
        &env.current_contract_address(),
        &pool.market_id,
        &outcome,
        &amount_in,
    );
    token_client.merge_positions(&env.current_contract_address(), &pool.market_id, &gross_out);
    DikeVaultClient::new(&env, &vault).release_trade_payout(
        &collateral,
        &trader,
        &pool.market_id,
        &outcome,
        &amount_in,
        &net_out,
    );
    DikeVaultClient::new(&env, &vault).collect_fee(
        &pool.market_id,
        &lp_fee,
        &treasury_fee,
        &cod_fee,
    );
    if yes {
        pool.yes_reserve = checked_sub(checked_add(pool.yes_reserve, amount_in)?, gross_out)?;
        pool.no_reserve = checked_sub(pool.no_reserve, gross_out)?;
    } else {
        pool.no_reserve = checked_sub(checked_add(pool.no_reserve, amount_in)?, gross_out)?;
        pool.yes_reserve = checked_sub(pool.yes_reserve, gross_out)?;
    }
    write_pool(&env, &pool);
    SellExecuted {
        pool_id,
        trader,
        yes,
        amount_in,
        amount_out: net_out,
    }
    .publish(&env);
    Ok(net_out)
}

#[contractimpl]
impl DikeAMM {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextPoolId, &1u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        bump(&env);
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

    pub fn set_modules(
        env: Env,
        vault: Address,
        tokens: Address,
        collateral: Address,
        registry: Address,
    ) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Vault, &vault);
        env.storage().instance().set(&DataKey::Tokens, &tokens);
        env.storage()
            .instance()
            .set(&DataKey::Collateral, &collateral);
        env.storage().instance().set(&DataKey::Registry, &registry);
        bump(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        Paused { paused }.publish(&env);
        Ok(())
    }

    pub fn create_pool(env: Env, market_id: u64, fee_config: FeeConfig) -> Result<u64, DikeError> {
        require_role(&env, symbol_short!("factory"))?;
        validate_fee_config(&fee_config)?;
        let pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextPoolId)
            .unwrap_or(1);
        let pool = PoolData {
            id: pool_id,
            market_id,
            yes_reserve: 0,
            no_reserve: 0,
            total_lp_shares: 0,
            accumulated_lp_fees: 0,
            accumulated_protocol_fees: 0,
            accumulated_cod_fees: 0,
            live: false,
        };
        write_pool(&env, &pool);
        write_fee(&env, pool_id, &fee_config);
        let next_pool_id = pool_id.checked_add(1).ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextPoolId, &next_pool_id);
        PoolCreated { market_id, pool_id }.publish(&env);
        Ok(pool_id)
    }

    pub fn seed_liquidity(
        env: Env,
        lp: Address,
        pool_id: u64,
        amount: i128,
    ) -> Result<i128, DikeError> {
        lp.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let (vault, tokens, collateral, _) = modules(&env)?;
        let mut pool = read_pool(&env, pool_id)?;
        require_market_seedable(&env, pool.market_id)?;
        if pool.total_lp_shares != 0 {
            return Err(DikeError::InvalidStatus);
        }
        DikeVaultClient::new(&env, &vault).deposit_for_market(
            &collateral,
            &lp,
            &pool.market_id,
            &amount,
        );
        DikeTokensClient::new(&env, &tokens).mint_complete_set(
            &env.current_contract_address(),
            &pool.market_id,
            &amount,
        );
        pool.yes_reserve = amount;
        pool.no_reserve = amount;
        pool.total_lp_shares = amount;
        pool.live = true;
        write_pool(&env, &pool);
        write_lp(&env, pool_id, lp.clone(), amount);
        LiquiditySeeded {
            pool_id,
            lp,
            amount,
        }
        .publish(&env);
        Ok(amount)
    }

    pub fn add_liquidity(
        env: Env,
        lp: Address,
        pool_id: u64,
        amount: i128,
    ) -> Result<i128, DikeError> {
        lp.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let (vault, tokens, collateral, _) = modules(&env)?;
        let mut pool = read_pool(&env, pool_id)?;
        require_market_tradeable(&env, pool.market_id)?;
        if !pool.live || pool.total_lp_shares <= 0 || pool.yes_reserve <= 0 {
            return Err(DikeError::InvalidStatus);
        }
        DikeVaultClient::new(&env, &vault).deposit_for_market(
            &collateral,
            &lp,
            &pool.market_id,
            &amount,
        );
        DikeTokensClient::new(&env, &tokens).mint_complete_set(
            &env.current_contract_address(),
            &pool.market_id,
            &amount,
        );
        let shares = proportional(pool.total_lp_shares, amount, pool.yes_reserve)?;
        pool.yes_reserve = checked_add(pool.yes_reserve, amount)?;
        pool.no_reserve = checked_add(pool.no_reserve, amount)?;
        pool.total_lp_shares = checked_add(pool.total_lp_shares, shares)?;
        write_pool(&env, &pool);
        let current = read_lp(&env, pool_id, lp.clone());
        write_lp(&env, pool_id, lp.clone(), checked_add(current, shares)?);
        LiquidityAdded {
            pool_id,
            lp,
            amount,
            shares,
        }
        .publish(&env);
        Ok(shares)
    }

    pub fn remove_liquidity(
        env: Env,
        lp: Address,
        pool_id: u64,
        shares: i128,
    ) -> Result<(i128, i128), DikeError> {
        lp.require_auth();
        if shares <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let (_, tokens, _, _) = modules(&env)?;
        let mut pool = read_pool(&env, pool_id)?;
        require_market_liquidity_removable(&env, pool.market_id)?;
        let current = read_lp(&env, pool_id, lp.clone());
        if current < shares || pool.total_lp_shares < shares {
            return Err(DikeError::InsufficientBalance);
        }
        let yes_out = proportional(pool.yes_reserve, shares, pool.total_lp_shares)?;
        let no_out = proportional(pool.no_reserve, shares, pool.total_lp_shares)?;
        pool.yes_reserve = checked_sub(pool.yes_reserve, yes_out)?;
        pool.no_reserve = checked_sub(pool.no_reserve, no_out)?;
        pool.total_lp_shares = checked_sub(pool.total_lp_shares, shares)?;
        write_pool(&env, &pool);
        write_lp(&env, pool_id, lp.clone(), checked_sub(current, shares)?);
        let token_client = DikeTokensClient::new(&env, &tokens);
        token_client.transfer_position(
            &env.current_contract_address(),
            &lp,
            &pool.market_id,
            &Outcome::Yes,
            &yes_out,
        );
        token_client.transfer_position(
            &env.current_contract_address(),
            &lp,
            &pool.market_id,
            &Outcome::No,
            &no_out,
        );
        LiquidityRemoved {
            pool_id,
            lp,
            shares,
            yes_out,
            no_out,
        }
        .publish(&env);
        Ok((yes_out, no_out))
    }

    pub fn buy_yes(
        env: Env,
        trader: Address,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        buy(
            env, trader, pool_id, amount_in, min_out, deadline, true, None,
        )
    }

    pub fn buy_no(
        env: Env,
        trader: Address,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        buy(
            env, trader, pool_id, amount_in, min_out, deadline, false, None,
        )
    }

    pub fn buy_child_yes(
        env: Env,
        trader: Address,
        parent_market_id: u64,
        parent_outcome: Outcome,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        buy(
            env,
            trader,
            pool_id,
            amount_in,
            min_out,
            deadline,
            true,
            Some((parent_market_id, parent_outcome)),
        )
    }

    pub fn buy_child_no(
        env: Env,
        trader: Address,
        parent_market_id: u64,
        parent_outcome: Outcome,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        buy(
            env,
            trader,
            pool_id,
            amount_in,
            min_out,
            deadline,
            false,
            Some((parent_market_id, parent_outcome)),
        )
    }

    pub fn sell_yes(
        env: Env,
        trader: Address,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        sell(env, trader, pool_id, amount_in, min_out, deadline, true)
    }

    pub fn sell_no(
        env: Env,
        trader: Address,
        pool_id: u64,
        amount_in: i128,
        min_out: i128,
        deadline: u64,
    ) -> Result<i128, DikeError> {
        sell(env, trader, pool_id, amount_in, min_out, deadline, false)
    }

    pub fn quote_buy_yes(env: Env, pool_id: u64, amount_in: i128) -> Result<TradeQuote, DikeError> {
        quote_buy_side(env, pool_id, amount_in, true)
    }

    pub fn quote_buy_no(env: Env, pool_id: u64, amount_in: i128) -> Result<TradeQuote, DikeError> {
        quote_buy_side(env, pool_id, amount_in, false)
    }

    pub fn pool(env: Env, pool_id: u64) -> Result<PoolData, DikeError> {
        read_pool(&env, pool_id)
    }

    pub fn lp_balance(env: Env, pool_id: u64, owner: Address) -> i128 {
        read_lp(&env, pool_id, owner)
    }
}

mod test;

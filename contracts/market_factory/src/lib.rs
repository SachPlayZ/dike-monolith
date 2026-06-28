#![no_std]

use dike_types::{DikeError, FeeConfig, MarketConfig, MarketData, MarketStatus};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, Address, Env,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Governance,
    Registry,
    Tokens,
    Vault,
    Amm,
    FeeManager,
    Creator(Address),
    Collateral(Address),
    MinLiquidity,
    MinExpiryDuration,
    NextMarketId,
    Market(u64),
    Paused,
}

#[contractevent(topics = ["modules"])]
#[derive(Clone)]
pub struct ModulesSet {}

#[contractevent(topics = ["creator"], data_format = "single-value")]
#[derive(Clone)]
pub struct CreatorSet {
    #[topic]
    pub creator: Address,
    pub approved: bool,
}

#[contractevent(topics = ["collat"], data_format = "single-value")]
#[derive(Clone)]
pub struct CollateralSet {
    #[topic]
    pub collateral: Address,
    pub supported: bool,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent(topics = ["mkt_new"], data_format = "vec")]
#[derive(Clone)]
pub struct MarketCreated {
    #[topic]
    pub market_id: u64,
    pub creator: Address,
    pub initial_liquidity: i128,
    pub opening_price_bps: u32,
}

#[contract]
pub struct DikeMarketFactory;

#[contractclient(name = "DikeRegistryClient")]
pub trait DikeRegistry {
    fn register_market(
        env: Env,
        config: MarketConfig,
        yes_token_id: u64,
        no_token_id: u64,
        pool_id: u64,
    ) -> Result<u64, DikeError>;
    fn activate_market(env: Env, market_id: u64) -> Result<(), DikeError>;
    fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError>;
}

#[contractclient(name = "DikeAmmClient")]
pub trait DikeAmm {
    fn create_pool(
        env: Env,
        market_id: u64,
        fee_config: dike_types::FeeConfig,
    ) -> Result<u64, DikeError>;
    fn seed_liquidity(env: Env, lp: Address, pool_id: u64, amount: i128)
        -> Result<i128, DikeError>;
}

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

fn require_governance(env: &Env) -> Result<(), DikeError> {
    let gov: Address = env
        .storage()
        .instance()
        .get(&DataKey::Governance)
        .ok_or(DikeError::Unauthorized)?;
    gov.require_auth();
    Ok(())
}

fn read_market(env: &Env, market_id: u64) -> Result<MarketData, DikeError> {
    let key = DataKey::Market(market_id);
    if !env.storage().persistent().has(&key) {
        return Err(DikeError::MarketNotFound);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(DikeError::MarketNotFound)
}

fn read_module(env: &Env, key: DataKey) -> Result<Address, DikeError> {
    env.storage()
        .instance()
        .get(&key)
        .ok_or(DikeError::NotInitialized)
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

fn validate(
    env: &Env,
    config: &MarketConfig,
    initial_liquidity: i128,
    opening_price_bps: u32,
) -> Result<(), DikeError> {
    if config.question.is_empty() || config.rules_uri.is_empty() {
        return Err(DikeError::InvalidInput);
    }
    let min_expiry: u64 = env
        .storage()
        .instance()
        .get(&DataKey::MinExpiryDuration)
        .unwrap_or(0);
    let earliest_expiry = env
        .ledger()
        .timestamp()
        .checked_add(min_expiry)
        .ok_or(DikeError::ArithmeticError)?;
    if config.expiry <= earliest_expiry {
        return Err(DikeError::InvalidInput);
    }
    if config.bond_amount <= 0 || config.dispute_window == 0 {
        return Err(DikeError::InvalidAmount);
    }
    validate_fee_config(&config.fee_config)?;
    let min_liquidity: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MinLiquidity)
        .unwrap_or(0);
    if initial_liquidity < min_liquidity || initial_liquidity <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    if opening_price_bps != 5_000 {
        return Err(DikeError::InvalidInput);
    }
    let creator_ok: bool = env
        .storage()
        .instance()
        .get(&DataKey::Creator(config.creator.clone()))
        .unwrap_or(false);
    if !creator_ok {
        return Err(DikeError::CreatorNotApproved);
    }
    let collateral_ok: bool = env
        .storage()
        .instance()
        .get(&DataKey::Collateral(config.collateral.clone()))
        .unwrap_or(false);
    if !collateral_ok {
        return Err(DikeError::UnsupportedCollateral);
    }
    Ok(())
}

#[contractimpl]
impl DikeMarketFactory {
    pub fn __constructor(
        env: Env,
        admin: Address,
        governance: Address,
        min_liquidity: i128,
        min_expiry_duration: u64,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
        env.storage()
            .instance()
            .set(&DataKey::MinLiquidity, &min_liquidity);
        env.storage()
            .instance()
            .set(&DataKey::MinExpiryDuration, &min_expiry_duration);
        env.storage().instance().set(&DataKey::NextMarketId, &1u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        bump(&env);
    }

    pub fn set_modules(
        env: Env,
        registry: Address,
        tokens: Address,
        vault: Address,
        amm: Address,
        fee_manager: Address,
    ) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Registry, &registry);
        env.storage().instance().set(&DataKey::Tokens, &tokens);
        env.storage().instance().set(&DataKey::Vault, &vault);
        env.storage().instance().set(&DataKey::Amm, &amm);
        env.storage()
            .instance()
            .set(&DataKey::FeeManager, &fee_manager);
        ModulesSet {}.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn set_creator(env: Env, creator: Address, approved: bool) -> Result<(), DikeError> {
        require_governance(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Creator(creator.clone()), &approved);
        CreatorSet { creator, approved }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn set_collateral(env: Env, collateral: Address, supported: bool) -> Result<(), DikeError> {
        require_governance(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Collateral(collateral.clone()), &supported);
        CollateralSet {
            collateral,
            supported,
        }
        .publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_governance(&env)?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        Paused { paused }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn create_market(
        env: Env,
        config: MarketConfig,
        initial_liquidity: i128,
        opening_price_bps: u32,
    ) -> Result<MarketData, DikeError> {
        config.creator.require_auth();
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(DikeError::InvalidStatus);
        }
        validate(&env, &config, initial_liquidity, opening_price_bps)?;
        let market_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextMarketId)
            .unwrap_or(1);
        let yes_token_id = market_id.checked_mul(2).ok_or(DikeError::ArithmeticError)?;
        let no_token_id = yes_token_id
            .checked_add(1)
            .ok_or(DikeError::ArithmeticError)?;
        let pool_id = market_id;
        let registry = read_module(&env, DataKey::Registry)?;
        let amm = read_module(&env, DataKey::Amm)?;
        let registry_client = DikeRegistryClient::new(&env, &registry);
        let amm_client = DikeAmmClient::new(&env, &amm);
        let registered_market_id =
            registry_client.register_market(&config, &yes_token_id, &no_token_id, &pool_id);
        if registered_market_id != market_id {
            return Err(DikeError::InvalidStatus);
        }
        let created_pool_id = amm_client.create_pool(&market_id, &config.fee_config);
        if created_pool_id != pool_id {
            return Err(DikeError::InvalidStatus);
        }
        amm_client.seed_liquidity(&config.creator, &pool_id, &initial_liquidity);
        registry_client.activate_market(&market_id);
        let mut market = registry_client.get_market(&market_id);
        market.status = MarketStatus::Live;
        let key = DataKey::Market(market_id);
        env.storage().persistent().set(&key, &market);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        let next_market_id = market_id.checked_add(1).ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextMarketId, &next_market_id);
        MarketCreated {
            market_id,
            creator: market.creator.clone(),
            initial_liquidity,
            opening_price_bps,
        }
        .publish(&env);
        Ok(market)
    }

    pub fn market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        read_market(&env, market_id)
    }
}

mod test;

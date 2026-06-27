#![no_std]

use dike_types::{DikeError, MarketConfig, MarketData, MarketId, MarketStatus};
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

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
    Market(MarketId),
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
    pub market_id: MarketId,
    pub creator: Address,
    pub initial_liquidity: i128,
    pub opening_price_bps: u32,
}

#[contract]
pub struct DikeMarketFactory;

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

fn read_market(env: &Env, market_id: MarketId) -> Result<MarketData, DikeError> {
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

fn validate(
    env: &Env,
    config: &MarketConfig,
    initial_liquidity: i128,
    opening_price_bps: u32,
) -> Result<(), DikeError> {
    if config.question.len() == 0 || config.rules_uri.len() == 0 {
        return Err(DikeError::InvalidInput);
    }
    let min_expiry: u64 = env
        .storage()
        .instance()
        .get(&DataKey::MinExpiryDuration)
        .unwrap_or(0);
    if config.expiry <= env.ledger().timestamp() + min_expiry {
        return Err(DikeError::InvalidInput);
    }
    if config.bond_amount <= 0 || config.dispute_window == 0 {
        return Err(DikeError::InvalidAmount);
    }
    let min_liquidity: i128 = env
        .storage()
        .instance()
        .get(&DataKey::MinLiquidity)
        .unwrap_or(0);
    if initial_liquidity < min_liquidity || initial_liquidity <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    if opening_price_bps == 0 || opening_price_bps >= 10_000 {
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
        let market_id: MarketId = env
            .storage()
            .instance()
            .get(&DataKey::NextMarketId)
            .unwrap_or(1);
        let yes_token_id = market_id * 2;
        let no_token_id = market_id * 2 + 1;
        let pool_id = market_id;
        let market = MarketData {
            id: market_id,
            question: config.question,
            question_hash: config.question_hash,
            rules_uri: config.rules_uri,
            rules_hash: config.rules_hash,
            creator: config.creator,
            collateral: config.collateral,
            yes_token_id,
            no_token_id,
            expiry: config.expiry,
            status: MarketStatus::Created,
            has_final_outcome: false,
            final_outcome: dike_types::Outcome::unset(),
            pool_id,
            bond_amount: config.bond_amount,
            dispute_window: config.dispute_window,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: config.fee_config,
        };
        let key = DataKey::Market(market_id);
        env.storage().persistent().set(&key, &market);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage()
            .instance()
            .set(&DataKey::NextMarketId, &(market_id + 1));
        MarketCreated {
            market_id,
            creator: market.creator.clone(),
            initial_liquidity,
            opening_price_bps,
        }
        .publish(&env);
        Ok(market)
    }

    pub fn market(env: Env, market_id: MarketId) -> Result<MarketData, DikeError> {
        read_market(&env, market_id)
    }
}

mod test;

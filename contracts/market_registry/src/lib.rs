#![no_std]

use dike_types::{
    DikeError, FeeConfig, MarketConfig, MarketData, MarketId, MarketStatus, Outcome, PoolId,
    RequestId,
};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    SupportedCollateral(Address),
    Market(MarketId),
    NextMarketId,
    Paused,
}

#[contract]
pub struct DikeMarketRegistry;

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);
}

fn require_admin(env: &Env) -> Result<Address, DikeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(DikeError::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}

fn require_role(env: &Env, role: Symbol) -> Result<Address, DikeError> {
    let module: Address = env
        .storage()
        .instance()
        .get(&DataKey::Role(role))
        .ok_or(DikeError::Unauthorized)?;
    module.require_auth();
    Ok(module)
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

fn write_market(env: &Env, market: &MarketData) {
    let key = DataKey::Market(market.id);
    env.storage().persistent().set(&key, market);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn valid_transition(from: MarketStatus, to: MarketStatus) -> bool {
    match (from, to) {
        (MarketStatus::Created, MarketStatus::Live) => true,
        (MarketStatus::Live, MarketStatus::Paused) => true,
        (MarketStatus::Paused, MarketStatus::Live) => true,
        (MarketStatus::Live, MarketStatus::TradingClosed) => true,
        (MarketStatus::TradingClosed, MarketStatus::ResolutionRequested) => true,
        (MarketStatus::ResolutionRequested, MarketStatus::Proposed) => true,
        (MarketStatus::Proposed, MarketStatus::Resolved) => true,
        (MarketStatus::Proposed, MarketStatus::Disputed) => true,
        (MarketStatus::Disputed, MarketStatus::CouncilVoting) => true,
        (MarketStatus::CouncilVoting, MarketStatus::Resolved) => true,
        (MarketStatus::Created, MarketStatus::Cancelled) => true,
        (MarketStatus::Live, MarketStatus::Cancelled) => true,
        (MarketStatus::Paused, MarketStatus::Cancelled) => true,
        _ => false,
    }
}

fn validate_config(env: &Env, config: &MarketConfig) -> Result<(), DikeError> {
    if config.question.len() == 0 || config.rules_uri.len() == 0 || config.category.len() == 0 {
        return Err(DikeError::InvalidInput);
    }
    if config.expiry <= env.ledger().timestamp() {
        return Err(DikeError::InvalidInput);
    }
    if config.bond_amount <= 0 || config.dispute_window == 0 {
        return Err(DikeError::InvalidAmount);
    }
    let supported = env
        .storage()
        .instance()
        .get(&DataKey::SupportedCollateral(config.collateral.clone()))
        .unwrap_or(false);
    if !supported {
        return Err(DikeError::UnsupportedCollateral);
    }
    Ok(())
}

fn transition_internal(
    env: &Env,
    market_id: MarketId,
    next: MarketStatus,
) -> Result<(), DikeError> {
    let mut market = read_market(env, market_id)?;
    if !valid_transition(market.status, next) {
        return Err(DikeError::InvalidTransition);
    }
    market.status = next;
    write_market(env, &market);
    env.events()
        .publish((symbol_short!("status"), market_id), next);
    Ok(())
}

#[contractimpl]
impl DikeMarketRegistry {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextMarketId, &1u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        bump(&env);
    }

    pub fn set_role(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Role(role.clone()), &module);
        env.events().publish((symbol_short!("role"), role), module);
        bump(&env);
        Ok(())
    }

    pub fn set_supported_collateral(
        env: Env,
        collateral: Address,
        supported: bool,
    ) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(
            &DataKey::SupportedCollateral(collateral.clone()),
            &supported,
        );
        env.events()
            .publish((symbol_short!("collat"), collateral), supported);
        bump(&env);
        Ok(())
    }

    pub fn pause_system(env: Env, paused: bool) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish((symbol_short!("pause"),), paused);
        bump(&env);
        Ok(())
    }

    pub fn register_market(
        env: Env,
        config: MarketConfig,
        yes_token_id: u64,
        no_token_id: u64,
        pool_id: PoolId,
    ) -> Result<MarketId, DikeError> {
        require_role(&env, symbol_short!("factory"))?;
        validate_config(&env, &config)?;
        let market_id: MarketId = env
            .storage()
            .instance()
            .get(&DataKey::NextMarketId)
            .unwrap_or(1);
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
            final_outcome: Outcome::unset(),
            pool_id,
            bond_amount: config.bond_amount,
            dispute_window: config.dispute_window,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: config.fee_config,
        };
        write_market(&env, &market);
        env.storage()
            .instance()
            .set(&DataKey::NextMarketId, &(market_id + 1));
        env.events()
            .publish((symbol_short!("mkt_new"), market_id), market.creator);
        bump(&env);
        Ok(market_id)
    }

    pub fn set_fee_config(
        env: Env,
        market_id: MarketId,
        fee_config: FeeConfig,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        let mut market = read_market(&env, market_id)?;
        if market.status == MarketStatus::Resolved || market.status == MarketStatus::Cancelled {
            return Err(DikeError::InvalidStatus);
        }
        market.fee_config = fee_config;
        write_market(&env, &market);
        env.events()
            .publish((symbol_short!("fee_cfg"), market_id), ());
        Ok(())
    }

    pub fn set_status(env: Env, market_id: MarketId, next: MarketStatus) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        let mut market = read_market(&env, market_id)?;
        if !valid_transition(market.status, next) {
            return Err(DikeError::InvalidTransition);
        }
        market.status = next;
        write_market(&env, &market);
        env.events()
            .publish((symbol_short!("status"), market_id), next);
        Ok(())
    }

    pub fn activate_market(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("factory"))?;
        transition_internal(&env, market_id, MarketStatus::Live)
    }

    pub fn close_trading(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        let market = read_market(&env, market_id)?;
        if env.ledger().timestamp() < market.expiry {
            return Err(DikeError::NotExpired);
        }
        transition_internal(&env, market_id, MarketStatus::TradingClosed)
    }

    pub fn mark_resolution_requested(
        env: Env,
        market_id: MarketId,
        request_id: RequestId,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        let mut market = read_market(&env, market_id)?;
        if !valid_transition(market.status, MarketStatus::ResolutionRequested) {
            return Err(DikeError::InvalidTransition);
        }
        market.status = MarketStatus::ResolutionRequested;
        market.has_request = true;
        market.request_id = request_id;
        write_market(&env, &market);
        env.events()
            .publish((symbol_short!("res_req"), market_id), request_id);
        Ok(())
    }

    pub fn mark_proposed(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::Proposed)
    }

    pub fn mark_disputed(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::Disputed)
    }

    pub fn mark_council_voting(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::CouncilVoting)
    }

    pub fn set_final_outcome(
        env: Env,
        market_id: MarketId,
        outcome: Outcome,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        let mut market = read_market(&env, market_id)?;
        if market.has_final_outcome || market.status == MarketStatus::Resolved {
            return Err(DikeError::AlreadyResolved);
        }
        if market.status != MarketStatus::Proposed && market.status != MarketStatus::CouncilVoting {
            return Err(DikeError::InvalidStatus);
        }
        market.has_final_outcome = true;
        market.final_outcome = outcome;
        market.status = MarketStatus::Resolved;
        write_market(&env, &market);
        env.events()
            .publish((symbol_short!("final"), market_id), outcome);
        Ok(())
    }

    pub fn cancel_market(env: Env, market_id: MarketId) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        transition_internal(&env, market_id, MarketStatus::Cancelled)
    }

    pub fn is_tradeable(env: Env, market_id: MarketId) -> Result<bool, DikeError> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        let market = read_market(&env, market_id)?;
        Ok(!paused
            && market.status == MarketStatus::Live
            && env.ledger().timestamp() < market.expiry)
    }

    pub fn get_market(env: Env, market_id: MarketId) -> Result<MarketData, DikeError> {
        read_market(&env, market_id)
    }

    pub fn get_status(env: Env, market_id: MarketId) -> Result<MarketStatus, DikeError> {
        Ok(read_market(&env, market_id)?.status)
    }

    pub fn get_final_outcome(env: Env, market_id: MarketId) -> Result<Outcome, DikeError> {
        let market = read_market(&env, market_id)?;
        if !market.has_final_outcome {
            return Err(DikeError::InvalidStatus);
        }
        Ok(market.final_outcome)
    }
}

mod test;

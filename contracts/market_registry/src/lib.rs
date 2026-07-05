#![no_std]

use dike_types::{
    validate_fee_config, DikeError, FeeConfig, MarketConfig, MarketData, MarketStatus, Outcome,
};
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    SupportedCollateral(Address),
    Market(u64),
    NextMarketId,
    Paused,
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

#[contractevent(topics = ["collat"], data_format = "single-value")]
#[derive(Clone)]
pub struct CollateralSupportSet {
    #[topic]
    pub collateral: Address,
    pub supported: bool,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct SystemPaused {
    pub paused: bool,
}

#[contractevent(topics = ["mkt_new"], data_format = "single-value")]
#[derive(Clone)]
pub struct MarketRegistered {
    #[topic]
    pub market_id: u64,
    pub creator: Address,
}

#[contractevent(topics = ["fee_cfg"])]
#[derive(Clone)]
pub struct MarketFeeConfigSet {
    #[topic]
    pub market_id: u64,
}

#[contractevent(topics = ["status"], data_format = "single-value")]
#[derive(Clone)]
pub struct MarketStatusSet {
    #[topic]
    pub market_id: u64,
    pub status: MarketStatus,
}

#[contractevent(topics = ["res_req"], data_format = "single-value")]
#[derive(Clone)]
pub struct ResolutionRequested {
    #[topic]
    pub market_id: u64,
    pub request_id: u64,
}

#[contractevent(topics = ["final"], data_format = "single-value")]
#[derive(Clone)]
pub struct FinalOutcomeSet {
    #[topic]
    pub market_id: u64,
    pub outcome: Outcome,
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

fn write_market(env: &Env, market: &MarketData) {
    let key = DataKey::Market(market.id);
    env.storage().persistent().set(&key, market);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn valid_transition(from: MarketStatus, to: MarketStatus) -> bool {
    matches!(
        (from, to),
        (MarketStatus::Created, MarketStatus::Live)
            | (MarketStatus::Live, MarketStatus::Paused)
            | (MarketStatus::Paused, MarketStatus::Live)
            | (MarketStatus::Live, MarketStatus::TradingClosed)
            | (MarketStatus::Paused, MarketStatus::TradingClosed)
            | (
                MarketStatus::TradingClosed,
                MarketStatus::ResolutionRequested
            )
            | (MarketStatus::ResolutionRequested, MarketStatus::Proposed)
            | (MarketStatus::Proposed, MarketStatus::Disputed)
            | (MarketStatus::Disputed, MarketStatus::CouncilVoting)
            | (MarketStatus::CouncilVoting, MarketStatus::Resolved)
            | (MarketStatus::Created, MarketStatus::Cancelled)
            | (MarketStatus::Live, MarketStatus::Cancelled)
            | (MarketStatus::Paused, MarketStatus::Cancelled)
    )
}

fn validate_config(env: &Env, config: &MarketConfig) -> Result<(), DikeError> {
    if config.question.is_empty() || config.rules_uri.is_empty() || config.category.is_empty() {
        return Err(DikeError::InvalidInput);
    }
    if config.expiry <= env.ledger().timestamp() {
        return Err(DikeError::InvalidInput);
    }
    if config.bond_amount <= 0 || config.dispute_window == 0 {
        return Err(DikeError::InvalidAmount);
    }
    validate_fee_config(&config.fee_config)?;
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

fn transition_internal(env: &Env, market_id: u64, next: MarketStatus) -> Result<(), DikeError> {
    let mut market = read_market(env, market_id)?;
    if !valid_transition(market.status, next) {
        return Err(DikeError::InvalidTransition);
    }
    market.status = next;
    write_market(env, &market);
    MarketStatusSet {
        market_id,
        status: next,
    }
    .publish(env);
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
        CollateralSupportSet {
            collateral,
            supported,
        }
        .publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn pause_system(env: Env, paused: bool) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        SystemPaused { paused }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn register_market(
        env: Env,
        config: MarketConfig,
        yes_token_id: u64,
        no_token_id: u64,
        pool_id: u64,
    ) -> Result<u64, DikeError> {
        require_role(&env, symbol_short!("factory"))?;
        validate_config(&env, &config)?;
        let market_id: u64 = env
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
        let next_market_id = market_id.checked_add(1).ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextMarketId, &next_market_id);
        MarketRegistered {
            market_id,
            creator: market.creator,
        }
        .publish(&env);
        bump(&env);
        Ok(market_id)
    }

    pub fn set_fee_config(
        env: Env,
        market_id: u64,
        fee_config: FeeConfig,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        validate_fee_config(&fee_config)?;
        let mut market = read_market(&env, market_id)?;
        if market.status == MarketStatus::Resolved || market.status == MarketStatus::Cancelled {
            return Err(DikeError::InvalidStatus);
        }
        market.fee_config = fee_config;
        write_market(&env, &market);
        MarketFeeConfigSet { market_id }.publish(&env);
        Ok(())
    }

    pub fn set_status(env: Env, market_id: u64, next: MarketStatus) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        let mut market = read_market(&env, market_id)?;
        if !valid_transition(market.status, next) {
            return Err(DikeError::InvalidTransition);
        }
        market.status = next;
        write_market(&env, &market);
        MarketStatusSet {
            market_id,
            status: next,
        }
        .publish(&env);
        Ok(())
    }

    pub fn activate_market(env: Env, market_id: u64) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("factory"))?;
        transition_internal(&env, market_id, MarketStatus::Live)
    }

    pub fn close_trading(env: Env, market_id: u64) -> Result<(), DikeError> {
        let market = read_market(&env, market_id)?;
        if env.ledger().timestamp() < market.expiry {
            return Err(DikeError::NotExpired);
        }
        transition_internal(&env, market_id, MarketStatus::TradingClosed)
    }

    pub fn mark_resolution_requested(
        env: Env,
        market_id: u64,
        request_id: u64,
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
        ResolutionRequested {
            market_id,
            request_id,
        }
        .publish(&env);
        Ok(())
    }

    pub fn mark_proposed(env: Env, market_id: u64) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::Proposed)
    }

    pub fn mark_disputed(env: Env, market_id: u64) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::Disputed)
    }

    pub fn mark_council_voting(env: Env, market_id: u64) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        transition_internal(&env, market_id, MarketStatus::CouncilVoting)
    }

    pub fn set_final_outcome(env: Env, market_id: u64, outcome: Outcome) -> Result<(), DikeError> {
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
        FinalOutcomeSet { market_id, outcome }.publish(&env);
        Ok(())
    }

    pub fn cancel_market(env: Env, market_id: u64) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        transition_internal(&env, market_id, MarketStatus::Cancelled)
    }

    pub fn is_tradeable(env: Env, market_id: u64) -> Result<bool, DikeError> {
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

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        read_market(&env, market_id)
    }

    pub fn get_status(env: Env, market_id: u64) -> Result<MarketStatus, DikeError> {
        Ok(read_market(&env, market_id)?.status)
    }

    pub fn get_final_outcome(env: Env, market_id: u64) -> Result<Outcome, DikeError> {
        let market = read_market(&env, market_id)?;
        if !market.has_final_outcome {
            return Err(DikeError::InvalidStatus);
        }
        Ok(market.final_outcome)
    }

    pub fn is_supported_collateral(env: Env, collateral: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::SupportedCollateral(collateral))
            .unwrap_or(false)
    }

    pub fn role(env: Env, role: Symbol) -> Result<Address, DikeError> {
        env.storage()
            .instance()
            .get(&DataKey::Role(role))
            .ok_or(DikeError::Unauthorized)
    }
}

mod test;

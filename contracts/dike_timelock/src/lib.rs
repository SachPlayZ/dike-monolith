#![no_std]

use dike_types::{DikeError, FeeConfig, TimelockAction, TimelockActionKind, TimelockPayload};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, xdr::ToXdr, Address,
    BytesN, Env, Symbol,
};

#[contractclient(name = "DikeGovernanceClient")]
pub trait DikeGovernance {
    fn apply_treasury(env: Env, treasury: Address) -> Result<(), DikeError>;
    fn apply_creator(env: Env, creator: Address, approved: bool) -> Result<(), DikeError>;
    fn apply_council_member(env: Env, member: Address, approved: bool) -> Result<(), DikeError>;
    fn apply_supported_collateral(
        env: Env,
        collateral: Address,
        supported: bool,
    ) -> Result<(), DikeError>;
    fn apply_module(env: Env, role: Symbol, module: Address) -> Result<(), DikeError>;
    fn apply_pause_authority(env: Env, authority: Address) -> Result<(), DikeError>;
    fn apply_fee_config(env: Env, config: FeeConfig) -> Result<(), DikeError>;
    fn record_upgrade_hash(
        env: Env,
        module_role: Symbol,
        wasm_hash: BytesN<32>,
    ) -> Result<(), DikeError>;
}

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Proposer,
    Executor,
    MinDelay,
    GracePeriod,
    NextActionId,
    Action(u64),
}

#[contractevent(topics = ["admin"], data_format = "single-value")]
#[derive(Clone)]
pub struct AdminSet {
    pub admin: Address,
}

#[contractevent(topics = ["roles"], data_format = "vec")]
#[derive(Clone)]
pub struct RolesSet {
    pub proposer: Address,
    pub executor: Address,
}

#[contractevent(topics = ["queued"], data_format = "vec")]
#[derive(Clone)]
pub struct ActionQueued {
    #[topic]
    pub action_id: u64,
    pub kind: TimelockActionKind,
    pub target: Address,
}

#[contractevent(topics = ["cancel"])]
#[derive(Clone)]
pub struct ActionCancelled {
    #[topic]
    pub action_id: u64,
}

#[contractevent(topics = ["execute"], data_format = "single-value")]
#[derive(Clone)]
pub struct ActionExecuted {
    #[topic]
    pub action_id: u64,
    pub kind: TimelockActionKind,
}

#[contract]
pub struct DikeTimelock;

fn require_address(env: &Env, key: DataKey) -> Result<(), DikeError> {
    let addr: Address = env
        .storage()
        .instance()
        .get(&key)
        .ok_or(DikeError::Unauthorized)?;
    addr.require_auth();
    Ok(())
}

fn read_action(env: &Env, action_id: u64) -> Result<TimelockAction, DikeError> {
    let key = DataKey::Action(action_id);
    if !env.storage().persistent().has(&key) {
        return Err(DikeError::ActionConsumed);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(DikeError::ActionConsumed)
}

fn write_action(env: &Env, action: &TimelockAction) {
    let key = DataKey::Action(action.id);
    env.storage().persistent().set(&key, action);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

#[contractimpl]
impl DikeTimelock {
    pub fn __constructor(
        env: Env,
        admin: Address,
        proposer: Address,
        executor: Address,
        min_delay: u64,
        grace_period: u64,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Proposer, &proposer);
        env.storage().instance().set(&DataKey::Executor, &executor);
        env.storage().instance().set(&DataKey::MinDelay, &min_delay);
        env.storage()
            .instance()
            .set(&DataKey::GracePeriod, &grace_period);
        env.storage().instance().set(&DataKey::NextActionId, &1u64);
    }

    pub fn set_admin(env: Env, admin: Address) -> Result<(), DikeError> {
        require_address(&env, DataKey::Admin)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        AdminSet { admin }.publish(&env);
        Ok(())
    }

    pub fn set_roles(env: Env, proposer: Address, executor: Address) -> Result<(), DikeError> {
        require_address(&env, DataKey::Admin)?;
        env.storage().instance().set(&DataKey::Proposer, &proposer);
        env.storage().instance().set(&DataKey::Executor, &executor);
        RolesSet { proposer, executor }.publish(&env);
        Ok(())
    }

    pub fn queue(
        env: Env,
        kind: TimelockActionKind,
        target: Address,
        payload: TimelockPayload,
        requested_delay: u64,
    ) -> Result<u64, DikeError> {
        require_address(&env, DataKey::Proposer)?;
        let min_delay: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MinDelay)
            .unwrap_or(0);
        if requested_delay < min_delay {
            return Err(DikeError::TimelockNotReady);
        }
        let grace: u64 = env
            .storage()
            .instance()
            .get(&DataKey::GracePeriod)
            .unwrap_or(0);
        let action_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextActionId)
            .unwrap_or(1);
        let execute_after = env
            .ledger()
            .timestamp()
            .checked_add(requested_delay)
            .ok_or(DikeError::ArithmeticError)?;
        let expires_at = execute_after
            .checked_add(grace)
            .ok_or(DikeError::ArithmeticError)?;
        let payload_hash = env
            .crypto()
            .sha256(&payload.clone().to_xdr(&env))
            .to_bytes();
        let action = TimelockAction {
            id: action_id,
            kind,
            target,
            payload,
            payload_hash,
            execute_after,
            expires_at,
            executed: false,
            cancelled: false,
        };
        write_action(&env, &action);
        let next_action_id = action_id.checked_add(1).ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextActionId, &next_action_id);
        ActionQueued {
            action_id,
            kind,
            target: action.target,
        }
        .publish(&env);
        Ok(action_id)
    }

    pub fn cancel(env: Env, action_id: u64) -> Result<(), DikeError> {
        require_address(&env, DataKey::Proposer)?;
        let mut action = read_action(&env, action_id)?;
        if action.executed || action.cancelled {
            return Err(DikeError::ActionConsumed);
        }
        action.cancelled = true;
        write_action(&env, &action);
        ActionCancelled { action_id }.publish(&env);
        Ok(())
    }

    pub fn execute(env: Env, action_id: u64) -> Result<TimelockAction, DikeError> {
        require_address(&env, DataKey::Executor)?;
        let mut action = read_action(&env, action_id)?;
        if action.executed || action.cancelled {
            return Err(DikeError::ActionConsumed);
        }
        let recomputed = env
            .crypto()
            .sha256(&action.payload.clone().to_xdr(&env))
            .to_bytes();
        if action.payload_hash != recomputed {
            return Err(DikeError::InvalidInput);
        }
        let now = env.ledger().timestamp();
        if now < action.execute_after {
            return Err(DikeError::TimelockNotReady);
        }
        if now > action.expires_at {
            return Err(DikeError::ActionConsumed);
        }

        let gov = DikeGovernanceClient::new(&env, &action.target);
        match action.payload.clone() {
            TimelockPayload::Treasury(addr) => {
                gov.apply_treasury(&addr);
            }
            TimelockPayload::Creator(addr, approved) => {
                gov.apply_creator(&addr, &approved);
            }
            TimelockPayload::CouncilMember(addr, approved) => {
                gov.apply_council_member(&addr, &approved);
            }
            TimelockPayload::SupportedCollateral(addr, supported) => {
                gov.apply_supported_collateral(&addr, &supported);
            }
            TimelockPayload::ModuleAddress(role, module) => {
                gov.apply_module(&role, &module);
            }
            TimelockPayload::Pause(authority) => {
                gov.apply_pause_authority(&authority);
            }
            TimelockPayload::FeeConfig(config) => {
                gov.apply_fee_config(&config);
            }
            TimelockPayload::Upgrade(role, wasm_hash) => {
                gov.record_upgrade_hash(&role, &wasm_hash);
            }
        }

        action.executed = true;
        write_action(&env, &action);
        ActionExecuted {
            action_id,
            kind: action.kind,
        }
        .publish(&env);
        Ok(action)
    }

    pub fn action(env: Env, action_id: u64) -> Result<TimelockAction, DikeError> {
        read_action(&env, action_id)
    }
}

mod test;

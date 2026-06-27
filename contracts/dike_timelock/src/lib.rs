#![no_std]

use dike_types::{ActionId, DikeError, TimelockAction, TimelockActionKind};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env};

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
    Action(ActionId),
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

fn read_action(env: &Env, action_id: ActionId) -> Result<TimelockAction, DikeError> {
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

    pub fn set_roles(env: Env, proposer: Address, executor: Address) -> Result<(), DikeError> {
        require_address(&env, DataKey::Admin)?;
        env.storage().instance().set(&DataKey::Proposer, &proposer);
        env.storage().instance().set(&DataKey::Executor, &executor);
        env.events()
            .publish((symbol_short!("roles"),), (proposer, executor));
        Ok(())
    }

    pub fn queue(
        env: Env,
        kind: TimelockActionKind,
        target: Address,
        payload_hash: BytesN<32>,
        requested_delay: u64,
    ) -> Result<ActionId, DikeError> {
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
        let action_id: ActionId = env
            .storage()
            .instance()
            .get(&DataKey::NextActionId)
            .unwrap_or(1);
        let execute_after = env.ledger().timestamp() + requested_delay;
        let action = TimelockAction {
            id: action_id,
            kind,
            target,
            payload_hash,
            execute_after,
            expires_at: execute_after + grace,
            executed: false,
            cancelled: false,
        };
        write_action(&env, &action);
        env.storage()
            .instance()
            .set(&DataKey::NextActionId, &(action_id + 1));
        env.events()
            .publish((symbol_short!("queued"), action_id), (kind, action.target));
        Ok(action_id)
    }

    pub fn cancel(env: Env, action_id: ActionId) -> Result<(), DikeError> {
        require_address(&env, DataKey::Proposer)?;
        let mut action = read_action(&env, action_id)?;
        if action.executed || action.cancelled {
            return Err(DikeError::ActionConsumed);
        }
        action.cancelled = true;
        write_action(&env, &action);
        env.events()
            .publish((symbol_short!("cancel"), action_id), ());
        Ok(())
    }

    pub fn execute(
        env: Env,
        action_id: ActionId,
        payload_hash: BytesN<32>,
    ) -> Result<TimelockAction, DikeError> {
        require_address(&env, DataKey::Executor)?;
        let mut action = read_action(&env, action_id)?;
        if action.executed || action.cancelled {
            return Err(DikeError::ActionConsumed);
        }
        if action.payload_hash != payload_hash {
            return Err(DikeError::InvalidInput);
        }
        let now = env.ledger().timestamp();
        if now < action.execute_after {
            return Err(DikeError::TimelockNotReady);
        }
        if now > action.expires_at {
            return Err(DikeError::ActionConsumed);
        }
        action.executed = true;
        write_action(&env, &action);
        env.events()
            .publish((symbol_short!("execute"), action_id), action.kind);
        Ok(action)
    }

    pub fn action(env: Env, action_id: ActionId) -> Result<TimelockAction, DikeError> {
        read_action(&env, action_id)
    }
}

mod test;

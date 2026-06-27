#![no_std]

use dike_math::{checked_add, checked_sub};
use dike_types::{DikeError, MarketId, Outcome};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Balance(MarketId, Address, Outcome),
    Backing(MarketId),
    Paused,
}

#[contract]
pub struct DikeConditionalTokens;

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

fn balance_key(market_id: MarketId, owner: Address, outcome: Outcome) -> DataKey {
    DataKey::Balance(market_id, owner, outcome)
}

fn read_balance(env: &Env, key: &DataKey) -> i128 {
    if !env.storage().persistent().has(key) {
        return 0;
    }
    env.storage()
        .persistent()
        .extend_ttl(key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(key).unwrap_or(0)
}

fn write_balance(env: &Env, key: &DataKey, amount: i128) {
    env.storage().persistent().set(key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(key, MIN_TTL, EXTEND_TTL);
}

fn add_balance(
    env: &Env,
    market_id: MarketId,
    owner: Address,
    outcome: Outcome,
    amount: i128,
) -> Result<(), DikeError> {
    let key = balance_key(market_id, owner, outcome);
    let next = checked_add(read_balance(env, &key), amount)?;
    write_balance(env, &key, next);
    Ok(())
}

fn sub_balance(
    env: &Env,
    market_id: MarketId,
    owner: Address,
    outcome: Outcome,
    amount: i128,
) -> Result<(), DikeError> {
    let key = balance_key(market_id, owner, outcome);
    let current = read_balance(env, &key);
    if current < amount {
        return Err(DikeError::InsufficientBalance);
    }
    write_balance(env, &key, checked_sub(current, amount)?);
    Ok(())
}

fn add_backing(env: &Env, market_id: MarketId, delta: i128) -> Result<(), DikeError> {
    let key = DataKey::Backing(market_id);
    let current = env.storage().persistent().get(&key).unwrap_or(0);
    let next = checked_add(current, delta)?;
    env.storage().persistent().set(&key, &next);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    Ok(())
}

fn sub_backing(env: &Env, market_id: MarketId, delta: i128) -> Result<(), DikeError> {
    let key = DataKey::Backing(market_id);
    let current = env.storage().persistent().get(&key).unwrap_or(0);
    if current < delta {
        return Err(DikeError::InsufficientCollateral);
    }
    let next = checked_sub(current, delta)?;
    env.storage().persistent().set(&key, &next);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    Ok(())
}

#[contractimpl]
impl DikeConditionalTokens {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
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

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish((symbol_short!("pause"),), paused);
        bump(&env);
        Ok(())
    }

    pub fn mint_complete_set(
        env: Env,
        to: Address,
        market_id: MarketId,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("vault"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        add_balance(&env, market_id, to.clone(), Outcome::Yes, amount)?;
        add_balance(&env, market_id, to.clone(), Outcome::No, amount)?;
        add_backing(&env, market_id, amount)?;
        env.events()
            .publish((symbol_short!("split"), market_id, to), amount);
        Ok(())
    }

    pub fn split_position(
        env: Env,
        user: Address,
        market_id: MarketId,
        amount: i128,
    ) -> Result<(), DikeError> {
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
        add_balance(&env, market_id, user.clone(), Outcome::Yes, amount)?;
        add_balance(&env, market_id, user.clone(), Outcome::No, amount)?;
        add_backing(&env, market_id, amount)?;
        env.events()
            .publish((symbol_short!("split"), market_id, user), amount);
        Ok(())
    }

    pub fn merge_positions(
        env: Env,
        user: Address,
        market_id: MarketId,
        amount: i128,
    ) -> Result<(), DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        sub_balance(&env, market_id, user.clone(), Outcome::Yes, amount)?;
        sub_balance(&env, market_id, user.clone(), Outcome::No, amount)?;
        sub_backing(&env, market_id, amount)?;
        env.events()
            .publish((symbol_short!("merge"), market_id, user), amount);
        Ok(())
    }

    pub fn transfer_position(
        env: Env,
        from: Address,
        to: Address,
        market_id: MarketId,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError> {
        from.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        sub_balance(&env, market_id, from.clone(), outcome, amount)?;
        add_balance(&env, market_id, to.clone(), outcome, amount)?;
        env.events().publish(
            (symbol_short!("pos_xfer"), market_id, from, to),
            (outcome, amount),
        );
        Ok(())
    }

    pub fn burn_for_redeem(
        env: Env,
        owner: Address,
        market_id: MarketId,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("vault"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        sub_balance(&env, market_id, owner.clone(), outcome, amount)?;
        env.events()
            .publish((symbol_short!("burn"), market_id, owner), (outcome, amount));
        Ok(())
    }

    pub fn burn_losing(
        env: Env,
        owner: Address,
        market_id: MarketId,
        outcome: Outcome,
        amount: i128,
    ) -> Result<(), DikeError> {
        owner.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        sub_balance(&env, market_id, owner.clone(), outcome, amount)?;
        env.events().publish(
            (symbol_short!("losebrn"), market_id, owner),
            (outcome, amount),
        );
        Ok(())
    }

    pub fn balance(env: Env, owner: Address, market_id: MarketId, outcome: Outcome) -> i128 {
        read_balance(&env, &balance_key(market_id, owner, outcome))
    }

    pub fn backing(env: Env, market_id: MarketId) -> i128 {
        let key = DataKey::Backing(market_id);
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

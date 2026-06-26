#![no_std]

use dike_types::DikeError;
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
}

#[contract]
pub struct MockUSDC;

fn require_admin(env: &Env) -> Result<(), DikeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(DikeError::NotInitialized)?;
    admin.require_auth();
    Ok(())
}

fn read_balance(env: &Env, owner: Address) -> i128 {
    let key = DataKey::Balance(owner);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage().persistent().get(&key).unwrap_or(0)
}

fn write_balance(env: &Env, owner: Address, amount: i128) {
    let key = DataKey::Balance(owner);
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

#[contractimpl]
impl MockUSDC {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);
    }

    pub fn decimals(_env: Env) -> u32 {
        7
    }

    pub fn name(env: Env) -> String {
        String::from_str(&env, "Mock USD Coin")
    }

    pub fn symbol(env: Env) -> String {
        String::from_str(&env, "USDC")
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, id)
    }

    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), DikeError> {
        require_admin(&env)?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let current = read_balance(&env, to.clone());
        write_balance(
            &env,
            to.clone(),
            current
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        let supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::TotalSupply,
            &supply
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        env.events().publish((symbol_short!("mint"), to), amount);
        Ok(())
    }

    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), DikeError> {
        from.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let current = read_balance(&env, from.clone());
        if current < amount {
            return Err(DikeError::InsufficientBalance);
        }
        write_balance(&env, from.clone(), current - amount);
        let supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(supply - amount));
        env.events().publish((symbol_short!("burn"), from), amount);
        Ok(())
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), DikeError> {
        from.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let from_balance = read_balance(&env, from.clone());
        if from_balance < amount {
            return Err(DikeError::InsufficientBalance);
        }
        let to_balance = read_balance(&env, to.clone());
        write_balance(&env, from.clone(), from_balance - amount);
        write_balance(
            &env,
            to.clone(),
            to_balance
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        env.events()
            .publish((symbol_short!("transfer"), from, to), amount);
        Ok(())
    }

    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        _expiration_ledger: u32,
    ) -> Result<(), DikeError> {
        from.require_auth();
        if amount < 0 {
            return Err(DikeError::InvalidAmount);
        }
        let key = DataKey::Allowance(from.clone(), spender.clone());
        env.storage().persistent().set(&key, &amount);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.events()
            .publish((symbol_short!("approve"), from, spender), amount);
        Ok(())
    }

    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let key = DataKey::Allowance(from, spender);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), DikeError> {
        spender.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let key = DataKey::Allowance(from.clone(), spender.clone());
        let allowance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if allowance < amount {
            return Err(DikeError::InsufficientBalance);
        }
        let from_balance = read_balance(&env, from.clone());
        if from_balance < amount {
            return Err(DikeError::InsufficientBalance);
        }
        let to_balance = read_balance(&env, to.clone());
        env.storage().persistent().set(&key, &(allowance - amount));
        write_balance(&env, from.clone(), from_balance - amount);
        write_balance(
            &env,
            to.clone(),
            to_balance
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        env.events()
            .publish((symbol_short!("xferfrom"), from, to), (spender, amount));
        Ok(())
    }
}

mod test;

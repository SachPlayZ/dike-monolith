#![no_std]

use dike_types::DikeError;
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, Address, BytesN, Env, String,
};

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

#[contractevent(topics = ["admin"], data_format = "single-value")]
#[derive(Clone)]
pub struct AdminSet {
    pub admin: Address,
}

#[contractevent(topics = ["mint"], data_format = "single-value")]
#[derive(Clone)]
pub struct Minted {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(topics = ["burn"], data_format = "single-value")]
#[derive(Clone)]
pub struct Burned {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent(topics = ["transfer"], data_format = "single-value")]
#[derive(Clone)]
pub struct Transferred {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(topics = ["approve"], data_format = "single-value")]
#[derive(Clone)]
pub struct Approved {
    #[topic]
    pub from: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
}

#[contractevent(topics = ["xferfrom"], data_format = "vec")]
#[derive(Clone)]
pub struct TransferredFrom {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub spender: Address,
    pub amount: i128,
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
    if !env.storage().persistent().has(&key) {
        return 0;
    }
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

    pub fn set_admin(env: Env, admin: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        AdminSet { admin }.publish(&env);
        Ok(())
    }

    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Ok(())
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
        Minted { to, amount }.publish(&env);
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
        write_balance(
            &env,
            from.clone(),
            current
                .checked_sub(amount)
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
                .checked_sub(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        Burned { from, amount }.publish(&env);
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
        write_balance(
            &env,
            from.clone(),
            from_balance
                .checked_sub(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        write_balance(
            &env,
            to.clone(),
            to_balance
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        Transferred { from, to, amount }.publish(&env);
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
        // The mock ignores expiration semantics and just stores the allowance amount.
        let key = DataKey::Allowance(from.clone(), spender.clone());
        env.storage().persistent().set(&key, &amount);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        Approved {
            from,
            spender,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let key = DataKey::Allowance(from, spender);
        if !env.storage().persistent().has(&key) {
            return 0;
        }
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
        env.storage().persistent().set(
            &key,
            &allowance
                .checked_sub(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        write_balance(
            &env,
            from.clone(),
            from_balance
                .checked_sub(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        write_balance(
            &env,
            to.clone(),
            to_balance
                .checked_add(amount)
                .ok_or(DikeError::ArithmeticError)?,
        );
        TransferredFrom {
            from,
            to,
            spender,
            amount,
        }
        .publish(&env);
        Ok(())
    }
}

mod test;

#![no_std]

use dike_math::{checked_add, checked_sub, invalid_refund};
use dike_types::{DikeError, MarketId, Outcome, RequestId, VaultAccounting};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token::Client as TokenClient, Address, Env,
    Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Treasury,
    Accounting(MarketId),
    Bond(RequestId, Address, bool),
    Redeemed(MarketId, Address, Outcome),
    Paused,
}

#[contract]
pub struct CollateralVault;

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);
}

fn zero_accounting() -> VaultAccounting {
    VaultAccounting {
        total_deposited: 0,
        collateral_backing: 0,
        amm_collateral: 0,
        redeemed: 0,
        protocol_fees: 0,
        lp_fees: 0,
        cod_fees: 0,
        proposal_bonds: 0,
        dispute_bonds: 0,
        refundable: 0,
    }
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

fn read_accounting(env: &Env, market_id: MarketId) -> VaultAccounting {
    let key = DataKey::Accounting(market_id);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(zero_accounting)
}

fn write_accounting(env: &Env, market_id: MarketId, accounting: &VaultAccounting) {
    let key = DataKey::Accounting(market_id);
    env.storage().persistent().set(&key, accounting);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn transfer_token(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    let client = TokenClient::new(env, token);
    client.transfer(from, to, &amount);
}

fn add_redeemed(
    env: &Env,
    market_id: MarketId,
    user: Address,
    outcome: Outcome,
    amount: i128,
) -> Result<(), DikeError> {
    let key = DataKey::Redeemed(market_id, user, outcome);
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    let next = checked_add(current, amount)?;
    env.storage().persistent().set(&key, &next);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    Ok(())
}

#[contractimpl]
impl CollateralVault {
    pub fn __constructor(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
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

    pub fn set_treasury(env: Env, treasury: Address) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.events().publish((symbol_short!("treas"),), treasury);
        bump(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish((symbol_short!("pause"),), paused);
        bump(&env);
        Ok(())
    }

    pub fn deposit_for_market(
        env: Env,
        token: Address,
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
        transfer_token(&env, &token, &user, &env.current_contract_address(), amount);
        let mut accounting = read_accounting(&env, market_id);
        accounting.total_deposited = checked_add(accounting.total_deposited, amount)?;
        accounting.collateral_backing = checked_add(accounting.collateral_backing, amount)?;
        accounting.refundable = checked_add(accounting.refundable, amount)?;
        write_accounting(&env, market_id, &accounting);
        env.events()
            .publish((symbol_short!("deposit"), market_id, user), amount);
        Ok(())
    }

    pub fn release_on_merge(
        env: Env,
        token: Address,
        user: Address,
        market_id: MarketId,
        amount: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("tokens"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let mut accounting = read_accounting(&env, market_id);
        if accounting.collateral_backing < amount || accounting.refundable < amount {
            return Err(DikeError::InsufficientCollateral);
        }
        accounting.collateral_backing = checked_sub(accounting.collateral_backing, amount)?;
        accounting.refundable = checked_sub(accounting.refundable, amount)?;
        write_accounting(&env, market_id, &accounting);
        transfer_token(&env, &token, &env.current_contract_address(), &user, amount);
        env.events()
            .publish((symbol_short!("release"), market_id, user), amount);
        Ok(())
    }

    pub fn redeem(
        env: Env,
        token: Address,
        user: Address,
        market_id: MarketId,
        final_outcome: Outcome,
        redeemed_outcome: Outcome,
        amount: i128,
    ) -> Result<i128, DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let payout = match final_outcome {
            Outcome::Invalid => invalid_refund(amount)?,
            Outcome::Yes => {
                if redeemed_outcome == Outcome::Yes {
                    amount
                } else {
                    0
                }
            }
            Outcome::No => {
                if redeemed_outcome == Outcome::No {
                    amount
                } else {
                    0
                }
            }
        };
        add_redeemed(&env, market_id, user.clone(), redeemed_outcome, amount)?;
        if payout > 0 {
            let mut accounting = read_accounting(&env, market_id);
            let max_remaining = checked_sub(accounting.collateral_backing, accounting.redeemed)?;
            if payout > max_remaining {
                return Err(DikeError::InsufficientCollateral);
            }
            accounting.redeemed = checked_add(accounting.redeemed, payout)?;
            accounting.refundable = checked_sub(accounting.refundable, payout)?;
            write_accounting(&env, market_id, &accounting);
            transfer_token(&env, &token, &env.current_contract_address(), &user, payout);
        }
        env.events().publish(
            (symbol_short!("redeem"), market_id, user),
            (redeemed_outcome, payout),
        );
        Ok(payout)
    }

    pub fn lock_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: RequestId,
        market_id: MarketId,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError> {
        user.require_auth();
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let bond_key = DataKey::Bond(request_id, user.clone(), is_dispute);
        if env.storage().persistent().has(&bond_key) {
            return Err(DikeError::InvalidInput);
        }
        transfer_token(&env, &token, &user, &env.current_contract_address(), amount);
        env.storage().persistent().set(&bond_key, &amount);
        env.storage()
            .persistent()
            .extend_ttl(&bond_key, MIN_TTL, EXTEND_TTL);
        let mut accounting = read_accounting(&env, market_id);
        if is_dispute {
            accounting.dispute_bonds = checked_add(accounting.dispute_bonds, amount)?;
        } else {
            accounting.proposal_bonds = checked_add(accounting.proposal_bonds, amount)?;
        }
        write_accounting(&env, market_id, &accounting);
        env.events().publish(
            (symbol_short!("bond"), request_id, user),
            (amount, is_dispute),
        );
        Ok(())
    }

    pub fn release_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: RequestId,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let bond_key = DataKey::Bond(request_id, user.clone(), is_dispute);
        let locked: i128 = env
            .storage()
            .persistent()
            .get(&bond_key)
            .ok_or(DikeError::InsufficientBalance)?;
        if locked < amount {
            return Err(DikeError::InsufficientBalance);
        }
        env.storage()
            .persistent()
            .set(&bond_key, &checked_sub(locked, amount)?);
        transfer_token(&env, &token, &env.current_contract_address(), &user, amount);
        env.events()
            .publish((symbol_short!("bond_rel"), request_id, user), amount);
        Ok(())
    }

    pub fn collect_fee(
        env: Env,
        market_id: MarketId,
        lp_fee: i128,
        protocol_fee: i128,
        cod_fee: i128,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("amm"))?;
        let mut accounting = read_accounting(&env, market_id);
        accounting.lp_fees = checked_add(accounting.lp_fees, lp_fee)?;
        accounting.protocol_fees = checked_add(accounting.protocol_fees, protocol_fee)?;
        accounting.cod_fees = checked_add(accounting.cod_fees, cod_fee)?;
        write_accounting(&env, market_id, &accounting);
        env.events().publish(
            (symbol_short!("fee"), market_id),
            (lp_fee, protocol_fee, cod_fee),
        );
        Ok(())
    }

    pub fn sweep_protocol_fees(
        env: Env,
        token: Address,
        market_id: MarketId,
    ) -> Result<i128, DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(DikeError::NotInitialized)?;
        let mut accounting = read_accounting(&env, market_id);
        let amount = checked_add(accounting.protocol_fees, accounting.cod_fees)?;
        accounting.protocol_fees = 0;
        accounting.cod_fees = 0;
        write_accounting(&env, market_id, &accounting);
        if amount > 0 {
            transfer_token(
                &env,
                &token,
                &env.current_contract_address(),
                &treasury,
                amount,
            );
        }
        Ok(amount)
    }

    pub fn accounting(env: Env, market_id: MarketId) -> VaultAccounting {
        read_accounting(&env, market_id)
    }

    pub fn redeemed(env: Env, market_id: MarketId, user: Address, outcome: Outcome) -> i128 {
        let key = DataKey::Redeemed(market_id, user, outcome);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}

mod test;

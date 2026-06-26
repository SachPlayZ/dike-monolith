#![no_std]

use dike_types::{DikeError, FeeConfig};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Timelock,
    Treasury,
    Creator(Address),
    CouncilMember(Address),
    SupportedCollateral(Address),
    Module(Symbol),
    PauseAuthority,
    FeeConfig,
    UpgradeHash(Symbol),
}

#[contract]
pub struct DikeGovernance;

fn require_admin(env: &Env) -> Result<(), DikeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(DikeError::NotInitialized)?;
    admin.require_auth();
    Ok(())
}

fn require_timelock(env: &Env) -> Result<(), DikeError> {
    let timelock: Address = env
        .storage()
        .instance()
        .get(&DataKey::Timelock)
        .ok_or(DikeError::Unauthorized)?;
    timelock.require_auth();
    Ok(())
}

#[contractimpl]
impl DikeGovernance {
    pub fn __constructor(env: Env, admin: Address, timelock: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Timelock, &timelock);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &FeeConfig::default());
    }

    pub fn set_timelock(env: Env, timelock: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Timelock, &timelock);
        env.events().publish((symbol_short!("timelock"),), timelock);
        Ok(())
    }

    pub fn apply_treasury(env: Env, treasury: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.events().publish((symbol_short!("treas"),), treasury);
        Ok(())
    }

    pub fn apply_creator(env: Env, creator: Address, approved: bool) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Creator(creator.clone()), &approved);
        env.events()
            .publish((symbol_short!("creator"), creator), approved);
        Ok(())
    }

    pub fn apply_council_member(
        env: Env,
        member: Address,
        approved: bool,
    ) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::CouncilMember(member.clone()), &approved);
        env.events()
            .publish((symbol_short!("member"), member), approved);
        Ok(())
    }

    pub fn apply_supported_collateral(
        env: Env,
        collateral: Address,
        supported: bool,
    ) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage().instance().set(
            &DataKey::SupportedCollateral(collateral.clone()),
            &supported,
        );
        env.events()
            .publish((symbol_short!("collat"), collateral), supported);
        Ok(())
    }

    pub fn apply_module(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Module(role.clone()), &module);
        env.events()
            .publish((symbol_short!("module"), role), module);
        Ok(())
    }

    pub fn apply_pause_authority(env: Env, authority: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::PauseAuthority, &authority);
        env.events().publish((symbol_short!("pauser"),), authority);
        Ok(())
    }

    pub fn apply_fee_config(env: Env, config: FeeConfig) -> Result<(), DikeError> {
        require_timelock(&env)?;
        if config.lp_fee_share_bps + config.treasury_fee_share_bps + config.cod_fee_share_bps
            != 10_000
        {
            return Err(DikeError::InvalidInput);
        }
        env.storage().instance().set(&DataKey::FeeConfig, &config);
        env.events().publish((symbol_short!("fee_cfg"),), ());
        Ok(())
    }

    pub fn record_upgrade_hash(
        env: Env,
        module_role: Symbol,
        wasm_hash: BytesN<32>,
    ) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::UpgradeHash(module_role.clone()), &wasm_hash);
        env.events()
            .publish((symbol_short!("upgrade"), module_role), wasm_hash);
        Ok(())
    }

    pub fn treasury(env: Env) -> Result<Address, DikeError> {
        env.storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(DikeError::NotInitialized)
    }

    pub fn is_creator(env: Env, creator: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Creator(creator))
            .unwrap_or(false)
    }

    pub fn is_council_member(env: Env, member: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::CouncilMember(member))
            .unwrap_or(false)
    }

    pub fn is_supported_collateral(env: Env, collateral: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::SupportedCollateral(collateral))
            .unwrap_or(false)
    }

    pub fn module(env: Env, role: Symbol) -> Result<Address, DikeError> {
        env.storage()
            .instance()
            .get(&DataKey::Module(role))
            .ok_or(DikeError::InvalidInput)
    }

    pub fn fee_config(env: Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_else(FeeConfig::default)
    }
}

mod test;

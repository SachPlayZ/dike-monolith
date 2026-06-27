#![no_std]

use dike_math::{bps, checked_sub, required_bond};
use dike_types::{
    DikeError, FeeConfig, DEFAULT_COUNCIL_BOND_SHARE_BPS, DEFAULT_TREASURY_BOND_SHARE_BPS,
    DEFAULT_WINNER_BOND_SHARE_BPS,
};
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Governance,
    Config,
    MinBond,
    BondBps,
    WinnerBondShareBps,
    CouncilBondShareBps,
    TreasuryBondShareBps,
}

#[contractevent(topics = ["fee_cfg"])]
#[derive(Clone)]
pub struct FeeConfigSet {}

#[contractevent(topics = ["bondcfg"], data_format = "vec")]
#[derive(Clone)]
pub struct BondConfigSet {
    pub minimum_bond: i128,
    pub bond_bps: u32,
}

#[contractevent(topics = ["bondspl"], data_format = "vec")]
#[derive(Clone)]
pub struct BondSplitSet {
    pub winner_bps: u32,
    pub council_bps: u32,
    pub treasury_bps: u32,
}

#[contract]
pub struct FeeManager;

fn require_governance(env: &Env) -> Result<(), DikeError> {
    let gov: Address = env
        .storage()
        .instance()
        .get(&DataKey::Governance)
        .ok_or(DikeError::Unauthorized)?;
    gov.require_auth();
    Ok(())
}

fn validate_fee_config(config: &FeeConfig) -> Result<(), DikeError> {
    if config.lp_fee_share_bps + config.treasury_fee_share_bps + config.cod_fee_share_bps != 10_000
    {
        return Err(DikeError::InvalidInput);
    }
    if config.trading_fee_bps > 1_000 {
        return Err(DikeError::InvalidInput);
    }
    Ok(())
}

#[contractimpl]
impl FeeManager {
    pub fn __constructor(
        env: Env,
        admin: Address,
        governance: Address,
        minimum_bond: i128,
        bond_bps: u32,
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
            .set(&DataKey::Config, &FeeConfig::default());
        env.storage()
            .instance()
            .set(&DataKey::MinBond, &minimum_bond);
        env.storage().instance().set(&DataKey::BondBps, &bond_bps);
        env.storage()
            .instance()
            .set(&DataKey::WinnerBondShareBps, &DEFAULT_WINNER_BOND_SHARE_BPS);
        env.storage().instance().set(
            &DataKey::CouncilBondShareBps,
            &DEFAULT_COUNCIL_BOND_SHARE_BPS,
        );
        env.storage().instance().set(
            &DataKey::TreasuryBondShareBps,
            &DEFAULT_TREASURY_BOND_SHARE_BPS,
        );
    }

    pub fn set_config(env: Env, config: FeeConfig) -> Result<(), DikeError> {
        require_governance(&env)?;
        validate_fee_config(&config)?;
        env.storage().instance().set(&DataKey::Config, &config);
        FeeConfigSet {}.publish(&env);
        Ok(())
    }

    pub fn set_bond_config(env: Env, minimum_bond: i128, bond_bps: u32) -> Result<(), DikeError> {
        require_governance(&env)?;
        if minimum_bond <= 0 || bond_bps > 10_000 {
            return Err(DikeError::InvalidInput);
        }
        env.storage()
            .instance()
            .set(&DataKey::MinBond, &minimum_bond);
        env.storage().instance().set(&DataKey::BondBps, &bond_bps);
        BondConfigSet {
            minimum_bond,
            bond_bps,
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_bond_split(
        env: Env,
        winner_bps: u32,
        council_bps: u32,
        treasury_bps: u32,
    ) -> Result<(), DikeError> {
        require_governance(&env)?;
        if winner_bps + council_bps + treasury_bps != 10_000 {
            return Err(DikeError::InvalidInput);
        }
        env.storage()
            .instance()
            .set(&DataKey::WinnerBondShareBps, &winner_bps);
        env.storage()
            .instance()
            .set(&DataKey::CouncilBondShareBps, &council_bps);
        env.storage()
            .instance()
            .set(&DataKey::TreasuryBondShareBps, &treasury_bps);
        BondSplitSet {
            winner_bps,
            council_bps,
            treasury_bps,
        }
        .publish(&env);
        Ok(())
    }

    pub fn config(env: Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap_or_else(FeeConfig::default)
    }

    pub fn required_bond(env: Env, market_liquidity: i128) -> Result<i128, DikeError> {
        let minimum_bond: i128 = env.storage().instance().get(&DataKey::MinBond).unwrap_or(0);
        let bond_bps: u32 = env.storage().instance().get(&DataKey::BondBps).unwrap_or(0);
        required_bond(minimum_bond, market_liquidity, bond_bps)
    }

    pub fn trading_fee(env: Env, amount: i128) -> Result<(i128, i128, i128, i128), DikeError> {
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let config = Self::config(env);
        let total_fee = bps(amount, config.trading_fee_bps)?;
        let net = checked_sub(amount, total_fee)?;
        let lp_fee = bps(total_fee, config.lp_fee_share_bps)?;
        let treasury_fee = bps(total_fee, config.treasury_fee_share_bps)?;
        let cod_fee = checked_sub(checked_sub(total_fee, lp_fee)?, treasury_fee)?;
        Ok((total_fee, net, lp_fee, treasury_fee + cod_fee))
    }

    pub fn trading_fee_split(
        env: Env,
        amount: i128,
    ) -> Result<(i128, i128, i128, i128), DikeError> {
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let config = Self::config(env);
        let total_fee = bps(amount, config.trading_fee_bps)?;
        let lp_fee = bps(total_fee, config.lp_fee_share_bps)?;
        let treasury_fee = bps(total_fee, config.treasury_fee_share_bps)?;
        let cod_fee = checked_sub(checked_sub(total_fee, lp_fee)?, treasury_fee)?;
        Ok((total_fee, lp_fee, treasury_fee, cod_fee))
    }

    pub fn losing_bond_split(env: Env, losing_bond: i128) -> Result<(i128, i128, i128), DikeError> {
        if losing_bond <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let winner_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::WinnerBondShareBps)
            .unwrap_or(DEFAULT_WINNER_BOND_SHARE_BPS);
        let council_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CouncilBondShareBps)
            .unwrap_or(DEFAULT_COUNCIL_BOND_SHARE_BPS);
        let winner = bps(losing_bond, winner_bps)?;
        let council = bps(losing_bond, council_bps)?;
        let treasury = checked_sub(checked_sub(losing_bond, winner)?, council)?;
        Ok((winner, council, treasury))
    }
}

mod test;

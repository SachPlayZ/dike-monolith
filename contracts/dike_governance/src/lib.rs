#![no_std]

use dike_types::{validate_fee_config, DikeError, FeeConfig};
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, Address, BytesN, Env, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

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

#[contractevent(topics = ["admin"], data_format = "single-value")]
#[derive(Clone)]
pub struct AdminSet {
    pub admin: Address,
}

#[contractevent(topics = ["timelock"], data_format = "single-value")]
#[derive(Clone)]
pub struct TimelockSet {
    pub timelock: Address,
}

#[contractevent(topics = ["treas"], data_format = "single-value")]
#[derive(Clone)]
pub struct TreasurySet {
    pub treasury: Address,
}

#[contractevent(topics = ["creator"], data_format = "single-value")]
#[derive(Clone)]
pub struct CreatorSet {
    #[topic]
    pub creator: Address,
    pub approved: bool,
}

#[contractevent(topics = ["member"], data_format = "single-value")]
#[derive(Clone)]
pub struct CouncilMemberSet {
    #[topic]
    pub member: Address,
    pub approved: bool,
}

#[contractevent(topics = ["collat"], data_format = "single-value")]
#[derive(Clone)]
pub struct SupportedCollateralSet {
    #[topic]
    pub collateral: Address,
    pub supported: bool,
}

#[contractevent(topics = ["module"], data_format = "single-value")]
#[derive(Clone)]
pub struct ModuleSet {
    #[topic]
    pub role: Symbol,
    pub module: Address,
}

#[contractevent(topics = ["pauser"], data_format = "single-value")]
#[derive(Clone)]
pub struct PauseAuthoritySet {
    pub authority: Address,
}

#[contractevent(topics = ["fee_cfg"])]
#[derive(Clone)]
pub struct FeeConfigSet {}

#[contractevent(topics = ["upgrade"], data_format = "single-value")]
#[derive(Clone)]
pub struct UpgradeHashRecorded {
    #[topic]
    pub module_role: Symbol,
    pub wasm_hash: BytesN<32>,
}

#[contract]
pub struct DikeGovernance;

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
    pub fn __constructor(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &FeeConfig::default());
        bump(&env);
    }

    pub fn set_admin(env: Env, admin: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        AdminSet { admin }.publish(&env);
        bump(&env);
        Ok(())
    }

    /// Bootstrap-once: sets the timelock address only when it has never been
    /// set before.  Once `DataKey::Timelock` is written, subsequent calls
    /// return `Err(AlreadyInitialized)`.  Further timelock address changes
    /// must go through `apply_timelock`, which is gated by `require_timelock`
    /// (i.e., through the timelock itself, enforcing the governance delay).
    pub fn set_timelock(env: Env, timelock: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        if env.storage().instance().has(&DataKey::Timelock) {
            return Err(DikeError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Timelock, &timelock);
        TimelockSet { timelock }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_timelock(env: Env, timelock: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage().instance().set(&DataKey::Timelock, &timelock);
        TimelockSet { timelock }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_treasury(env: Env, treasury: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        TreasurySet { treasury }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_creator(env: Env, creator: Address, approved: bool) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Creator(creator.clone()), &approved);
        CreatorSet { creator, approved }.publish(&env);
        bump(&env);
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
        CouncilMemberSet { member, approved }.publish(&env);
        bump(&env);
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
        SupportedCollateralSet {
            collateral,
            supported,
        }
        .publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_module(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Module(role.clone()), &module);
        ModuleSet { role, module }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_pause_authority(env: Env, authority: Address) -> Result<(), DikeError> {
        require_timelock(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::PauseAuthority, &authority);
        PauseAuthoritySet { authority }.publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn apply_fee_config(env: Env, config: FeeConfig) -> Result<(), DikeError> {
        require_timelock(&env)?;
        validate_fee_config(&config)?;
        env.storage().instance().set(&DataKey::FeeConfig, &config);
        FeeConfigSet {}.publish(&env);
        bump(&env);
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
        UpgradeHashRecorded {
            module_role,
            wasm_hash,
        }
        .publish(&env);
        bump(&env);
        Ok(())
    }

    pub fn treasury(env: Env) -> Result<Address, DikeError> {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(DikeError::NotInitialized)
    }

    pub fn timelock(env: Env) -> Result<Address, DikeError> {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::Timelock)
            .ok_or(DikeError::NotInitialized)
    }

    pub fn is_creator(env: Env, creator: Address) -> bool {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::Creator(creator))
            .unwrap_or(false)
    }

    pub fn is_council_member(env: Env, member: Address) -> bool {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::CouncilMember(member))
            .unwrap_or(false)
    }

    pub fn is_supported_collateral(env: Env, collateral: Address) -> bool {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::SupportedCollateral(collateral))
            .unwrap_or(false)
    }

    pub fn module(env: Env, role: Symbol) -> Result<Address, DikeError> {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::Module(role))
            .ok_or(DikeError::InvalidInput)
    }

    pub fn fee_config(env: Env) -> FeeConfig {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_default()
    }

    pub fn pause_authority(env: Env) -> Result<Address, DikeError> {
        bump(&env);
        env.storage()
            .instance()
            .get(&DataKey::PauseAuthority)
            .ok_or(DikeError::NotInitialized)
    }
}

mod test;

#![cfg(test)]

use super::*;
use dike_governance::{DikeGovernance, DikeGovernanceClient as RealGovernanceClient};
use fee_manager::{FeeManager, FeeManagerClient};
use market_factory::{DikeMarketFactory, DikeMarketFactoryClient};
use market_registry::{DikeMarketRegistry, DikeMarketRegistryClient};
use council_of_dike::{CouncilOfDike, CouncilOfDikeClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    symbol_short,
    Env,
};

fn setup_governance<'a>(env: &'a Env, timelock: &Address) -> (Address, RealGovernanceClient<'a>) {
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    let gov_id = env.register(DikeGovernance, (&admin, &treasury));
    let gov = RealGovernanceClient::new(env, &gov_id);
    gov.set_timelock(timelock);
    (gov_id, gov)
}

#[test]
fn queues_and_executes_after_delay_and_calls_target_module_directly() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, gov) = setup_governance(&env, &id);
    let factory_admin = Address::generate(&env);
    let factory_id = env.register(DikeMarketFactory, (&factory_admin, &gov_id, &1i128, &1u64));
    let factory = DikeMarketFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    let payload = TimelockPayload::Creator(creator.clone(), true);
    let action_id = client.queue(&TimelockActionKind::Creator, &factory_id, &payload, &10);
    assert!(!factory.is_creator(&creator));
    assert!(client.try_execute(&action_id).is_err());
    env.ledger().set_timestamp(111);
    let action = client.execute(&action_id);
    assert!(action.executed);
    assert!(factory.is_creator(&creator));
    assert!(!gov.is_creator(&creator));
    // Replaying is rejected once already executed.
    assert!(client.try_execute(&action_id).is_err());
}

#[test]
fn queue_rejects_kind_payload_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, _gov) = setup_governance(&env, &id);

    // A proposer cannot label a Timelock-rotation payload as a benign Creator kind.
    let attacker = Address::generate(&env);
    let payload = TimelockPayload::Timelock(attacker);
    assert!(matches!(
        client.try_queue(&TimelockActionKind::Creator, &gov_id, &payload, &10),
        Err(Ok(DikeError::InvalidInput))
    ));
}

#[test]
fn executes_treasury_and_fee_config_payloads() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, gov) = setup_governance(&env, &id);
    let fee_admin = Address::generate(&env);
    let fee_manager_id = env.register(FeeManager, (&fee_admin, &id, &1i128, &100u32));
    let fee_manager = FeeManagerClient::new(&env, &fee_manager_id);

    let new_treasury = Address::generate(&env);
    let treasury_action = client.queue(
        &TimelockActionKind::Treasury,
        &gov_id,
        &TimelockPayload::Treasury(new_treasury.clone()),
        &10,
    );
    let mut new_fee_config = dike_types::FeeConfig::default();
    new_fee_config.trading_fee_bps = 50;
    let fee_action = client.queue(
        &TimelockActionKind::FeeConfig,
        &fee_manager_id,
        &TimelockPayload::FeeConfig(new_fee_config.clone()),
        &10,
    );
    env.ledger().set_timestamp(111);
    client.execute(&treasury_action);
    client.execute(&fee_action);
    assert_eq!(gov.treasury(), new_treasury);
    assert_eq!(fee_manager.config().trading_fee_bps, 50);
    assert_eq!(gov.fee_config().trading_fee_bps, FeeConfig::default().trading_fee_bps);
}

#[test]
fn executes_supported_collateral_and_module_updates_on_target_contracts() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let registry_id = env.register(DikeMarketRegistry, (&id,));
    let registry = DikeMarketRegistryClient::new(&env, &registry_id);
    let council_id = env.register(CouncilOfDike, (&id,));
    let council = CouncilOfDikeClient::new(&env, &council_id);

    council.set_role(&symbol_short!("gov"), &id);

    let collateral = Address::generate(&env);
    let module = Address::generate(&env);
    let member = Address::generate(&env);
    let collateral_action = client.queue(
        &TimelockActionKind::SupportedCollateral,
        &registry_id,
        &TimelockPayload::SupportedCollateral(collateral.clone(), true),
        &10,
    );
    let module_action = client.queue(
        &TimelockActionKind::ModuleAddress,
        &registry_id,
        &TimelockPayload::ModuleAddress(symbol_short!("oracle"), module.clone()),
        &10,
    );
    let member_action = client.queue(
        &TimelockActionKind::CouncilMember,
        &council_id,
        &TimelockPayload::CouncilMember(member.clone(), true),
        &10,
    );
    env.ledger().set_timestamp(111);
    client.execute(&collateral_action);
    client.execute(&module_action);
    client.execute(&member_action);

    assert!(registry.is_supported_collateral(&collateral));
    assert_eq!(registry.role(&symbol_short!("oracle")), module);
    assert!(council.is_member(&member));
}

#[test]
fn rejects_overflowing_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(u64::MAX - 5);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let target = Address::generate(&env);
    let id = env.register(DikeTimelock, (&admin, &proposer, &executor, &1u64, &100u64));
    let client = DikeTimelockClient::new(&env, &id);
    let payload = TimelockPayload::Pause(Address::generate(&env));
    assert!(matches!(
        client.try_queue(&TimelockActionKind::Pause, &target, &payload, &10),
        Err(Ok(DikeError::ArithmeticError))
    ));
}

#[test]
fn execute_before_ready_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, _gov) = setup_governance(&env, &id);
    let payload = TimelockPayload::Pause(Address::generate(&env));
    let action_id = client.queue(&TimelockActionKind::Pause, &gov_id, &payload, &10);
    assert!(matches!(
        client.try_execute(&action_id),
        Err(Ok(DikeError::TimelockNotReady))
    ));
}

#[test]
fn execute_after_expiry_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(DikeTimelock, (&admin, &proposer, &executor, &10u64, &5u64));
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, _gov) = setup_governance(&env, &id);
    let payload = TimelockPayload::Pause(Address::generate(&env));
    let action_id = client.queue(&TimelockActionKind::Pause, &gov_id, &payload, &10);
    env.ledger().set_timestamp(200);
    assert!(matches!(
        client.try_execute(&action_id),
        Err(Ok(DikeError::ActionConsumed))
    ));
}

#[test]
fn cancelled_action_cannot_execute() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, _gov) = setup_governance(&env, &id);
    let payload = TimelockPayload::Pause(Address::generate(&env));
    let action_id = client.queue(&TimelockActionKind::Pause, &gov_id, &payload, &10);
    client.cancel(&action_id);
    env.ledger().set_timestamp(111);
    assert!(matches!(
        client.try_execute(&action_id),
        Err(Ok(DikeError::ActionConsumed))
    ));
}

// --- Item 5: Timelock payload rotates governance's timelock address ---

#[test]
fn executes_timelock_payload_to_rotate_governance_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(DikeTimelock, (&admin, &proposer, &executor, &10u64, &100u64));
    let client = DikeTimelockClient::new(&env, &id);
    let (gov_id, gov) = setup_governance(&env, &id);
    let new_timelock = Address::generate(&env);
    let action_id = client.queue(
        &TimelockActionKind::Timelock,
        &gov_id,
        &TimelockPayload::Timelock(new_timelock.clone()),
        &10,
    );
    env.ledger().set_timestamp(111);
    client.execute(&action_id);
    assert_eq!(gov.timelock(), new_timelock);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths so require_auth panics for wrong signer
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(DikeTimelock, (&admin, &proposer, &executor, &10u64, &100u64));
    let client = DikeTimelockClient::new(&env, &id);
    let payload = TimelockPayload::Pause(Address::generate(&env));
    let target = Address::generate(&env);
    // queue needs proposer auth — without it, require_auth panics
    assert!(client
        .try_queue(&TimelockActionKind::Pause, &target, &payload, &10)
        .is_err());
    // cancel and execute also require roles
    assert!(client.try_cancel(&1).is_err());
    assert!(client.try_execute(&1).is_err());
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let id = env.register(DikeTimelock, (&admin, &proposer, &executor, &10u64, &100u64));
    let client = DikeTimelockClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_roles(&other, &other).is_err());
}

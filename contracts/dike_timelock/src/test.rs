#![cfg(test)]

use super::*;
use dike_governance::{DikeGovernance, DikeGovernanceClient as RealGovernanceClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Env,
};

fn setup_governance<'a>(env: &'a Env, timelock: &Address) -> (Address, RealGovernanceClient<'a>) {
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    let gov_id = env.register(DikeGovernance, (&admin, timelock, &treasury));
    let gov = RealGovernanceClient::new(env, &gov_id);
    (gov_id, gov)
}

#[test]
fn queues_and_executes_after_delay_and_actually_applies_governance_change() {
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

    let creator = Address::generate(&env);
    let payload = TimelockPayload::Creator(creator.clone(), true);
    let action_id = client.queue(&TimelockActionKind::Creator, &gov_id, &payload, &10);
    assert!(!gov.is_creator(&creator));
    assert!(client.try_execute(&action_id).is_err());
    env.ledger().set_timestamp(111);
    let action = client.execute(&action_id);
    assert!(action.executed);
    assert!(gov.is_creator(&creator));
    // Replaying is rejected once already executed.
    assert!(client.try_execute(&action_id).is_err());
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
        &gov_id,
        &TimelockPayload::FeeConfig(new_fee_config.clone()),
        &10,
    );
    env.ledger().set_timestamp(111);
    client.execute(&treasury_action);
    client.execute(&fee_action);
    assert_eq!(gov.treasury(), new_treasury);
    assert_eq!(gov.fee_config().trading_fee_bps, 50);
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


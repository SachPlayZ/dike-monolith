#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

fn setup(env: &Env) -> (DikeGovernanceClient, Address, Address) {
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    let id = env.register(DikeGovernance, (&admin, &treasury));
    let client = DikeGovernanceClient::new(env, &id);
    (client, admin, treasury)
}

#[test]
fn set_timelock_bootstrap_once_then_locked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _treasury) = setup(&env);
    let timelock = Address::generate(&env);
    // first call should succeed
    assert!(client.try_set_timelock(&timelock).is_ok());
    assert_eq!(client.timelock(), timelock);
    // second call should fail with AlreadyInitialized
    assert!(matches!(
        client.try_set_timelock(&Address::generate(&env)),
        Err(Ok(DikeError::AlreadyInitialized))
    ));
}

#[test]
fn apply_timelock_requires_current_timelock_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _treasury) = setup(&env);
    let timelock = Address::generate(&env);
    client.set_timelock(&timelock);
    let new_timelock = Address::generate(&env);
    // apply_timelock is gated by require_timelock
    assert!(client.try_apply_timelock(&new_timelock).is_ok());
    assert_eq!(client.timelock(), new_timelock);
}

#[test]
fn timelock_applies_creator_and_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, treasury) = setup(&env);
    let timelock = Address::generate(&env);
    let pauser = Address::generate(&env);
    let creator = Address::generate(&env);
    let collateral = Address::generate(&env);
    client.set_timelock(&timelock);

    client.apply_creator(&creator, &true);
    client.apply_supported_collateral(&collateral, &true);
    client.apply_pause_authority(&pauser);
    assert!(client.is_creator(&creator));
    assert!(client.is_supported_collateral(&collateral));
    assert_eq!(client.timelock(), timelock);
    assert_eq!(client.treasury(), treasury);
    assert_eq!(client.pause_authority(), pauser);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn set_timelock_rejects_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let id = env.register(DikeGovernance, (&admin, &treasury));
    let client = DikeGovernanceClient::new(&env, &id);
    // No admin auth in context → require_admin panics
    assert!(client.try_set_timelock(&Address::generate(&env)).is_err());
}

#[test]
fn apply_treasury_rejects_unconfigured_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _treasury) = setup(&env);
    // timelock not set yet → require_timelock returns Unauthorized
    assert!(matches!(
        client.try_apply_treasury(&Address::generate(&env)),
        Err(Ok(DikeError::Unauthorized))
    ));
}

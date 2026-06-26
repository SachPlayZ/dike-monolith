#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn timelock_applies_creator_and_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let timelock = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    let collateral = Address::generate(&env);
    let id = env.register(DikeGovernance, (&admin, &timelock, &treasury));
    let client = DikeGovernanceClient::new(&env, &id);

    client.apply_creator(&creator, &true);
    client.apply_supported_collateral(&collateral, &true);
    assert!(client.is_creator(&creator));
    assert!(client.is_supported_collateral(&collateral));
}

#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn mints_and_transfers() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let id = env.register(MockUSDC, (&admin,));
    let client = MockUSDCClient::new(&env, &id);
    client.mint(&alice, &1_000);
    client.transfer(&alice, &bob, &250);
    assert_eq!(client.balance(&alice), 750);
    assert_eq!(client.balance(&bob), 250);
}

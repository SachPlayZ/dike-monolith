#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn calculates_default_fee_split() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let id = env.register(FeeManager, (&admin, &gov, &500i128, &100u32));
    let client = FeeManagerClient::new(&env, &id);

    let (total, lp, treasury, cod) = client.trading_fee_split(&10_000);
    assert_eq!(total, 200);
    assert_eq!(lp, 140);
    assert_eq!(treasury, 40);
    assert_eq!(cod, 20);
    assert_eq!(client.required_bond(&100_000), 1_000);
}

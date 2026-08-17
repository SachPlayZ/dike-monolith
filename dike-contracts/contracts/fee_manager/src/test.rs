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

#[test]
#[should_panic(expected = "invalid bond config")]
fn constructor_rejects_invalid_bond_config() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);

    let _id = env.register(FeeManager, (&admin, &gov, &0i128, &10_001u32));
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn admin_and_governance_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let id = env.register(FeeManager, (&admin, &gov, &500i128, &100u32));
    let client = FeeManagerClient::new(&env, &id);
    let other = Address::generate(&env);
    // set_admin gated by admin.require_auth()
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_governance(&other).is_err());
    // set_config gated by gov.require_auth()
    assert!(client.try_set_config(&FeeConfig::default()).is_err());
}

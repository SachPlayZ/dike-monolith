#![cfg(test)]

use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Env};

#[contract]
pub struct AllowingVault;

#[contractimpl]
impl AllowingVault {
    pub fn assert_position_transfer_allowed(
        _env: Env,
        _from: Address,
        _market_id: u64,
        _outcome: Outcome,
        _amount: i128,
    ) -> Result<(), DikeError> {
        Ok(())
    }
}

#[test]
fn split_merge_and_transfer_positions() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let vault_id = env.register(AllowingVault, ());
    let id = env.register(DikeConditionalTokens, (&admin,));
    let client = DikeConditionalTokensClient::new(&env, &id);
    client.set_role(&symbol_short!("vault"), &vault_id);
    client.set_role(&symbol_short!("amm"), &alice);

    client.split_position(&alice, &1, &100);
    assert_eq!(client.balance(&alice, &1, &Outcome::Yes), 100);
    assert_eq!(client.balance(&alice, &1, &Outcome::No), 100);
    assert_eq!(client.backing(&1), 100);

    client.transfer_position(&alice, &bob, &1, &Outcome::Yes, &40);
    assert_eq!(client.balance(&alice, &1, &Outcome::Yes), 60);
    assert_eq!(client.balance(&bob, &1, &Outcome::Yes), 40);

    client.merge_positions(&alice, &1, &20);
    assert_eq!(client.balance(&alice, &1, &Outcome::Yes), 40);
    assert_eq!(client.balance(&alice, &1, &Outcome::No), 80);
    assert_eq!(client.backing(&1), 80);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let id = env.register(DikeConditionalTokens, (&admin,));
    let client = DikeConditionalTokensClient::new(&env, &id);
    let user = Address::generate(&env);
    // amm role not configured
    assert!(matches!(
        client.try_mint_complete_set(&user, &1, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_merge_positions(&user, &1, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
    // vault role not configured
    assert!(matches!(
        client.try_split_position(&user, &1, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_burn_for_redeem(&user, &1, &Outcome::Yes, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let id = env.register(DikeConditionalTokens, (&admin,));
    let client = DikeConditionalTokensClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client
        .try_set_role(&symbol_short!("amm"), &other)
        .is_err());
    assert!(client.try_pause(&true).is_err());
}

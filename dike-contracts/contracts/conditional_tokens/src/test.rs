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

#[contract]
pub struct BlockingVault;

#[contractimpl]
impl BlockingVault {
    pub fn assert_position_transfer_allowed(
        _env: Env,
        _from: Address,
        _market_id: u64,
        _outcome: Outcome,
        _amount: i128,
    ) -> Result<(), DikeError> {
        Err(DikeError::EncumberedPosition)
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

#[test]
fn transfer_position_forced_moves_balance_and_skips_encumbrance_guard() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    // A vault stub that ALWAYS rejects transfers (simulates an encumbered
    // position) — proves transfer_position_forced skips this callback
    // entirely, unlike voluntary transfer_position which would fail here.
    let vault_id = env.register(BlockingVault, ());
    let id = env.register(DikeConditionalTokens, (&admin,));
    let client = DikeConditionalTokensClient::new(&env, &id);
    client.set_role(&symbol_short!("vault"), &vault_id);
    client.set_role(&symbol_short!("amm"), &alice);

    client.split_position(&alice, &1, &100);
    // Voluntary transfer is blocked by the vault's encumbrance guard.
    assert!(matches!(
        client.try_transfer_position(&alice, &bob, &1, &Outcome::Yes, &40),
        Err(Ok(DikeError::EncumberedPosition))
    ));

    // Forced transfer (role-gated "amm", called by alice acting as the amm
    // module) moves the balance anyway — no owner signature and no
    // encumbrance check, matching how liquidation force-closes a position
    // `assert_position_transfer_allowed` would otherwise block.
    client.transfer_position_forced(&alice, &bob, &1, &Outcome::Yes, &40);
    assert_eq!(client.balance(&alice, &1, &Outcome::Yes), 60);
    assert_eq!(client.balance(&bob, &1, &Outcome::Yes), 40);
}

#[test]
fn transfer_position_forced_rejects_unconfigured_amm_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let bob = Address::generate(&env);
    let id = env.register(DikeConditionalTokens, (&admin,));
    let client = DikeConditionalTokensClient::new(&env, &id);
    // "amm" role never configured
    assert!(matches!(
        client.try_transfer_position_forced(&bob, &admin, &1, &Outcome::Yes, &1),
        Err(Ok(DikeError::Unauthorized))
    ));
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
    assert!(client.try_set_role(&symbol_short!("amm"), &other).is_err());
    assert!(client.try_pause(&true).is_err());
}

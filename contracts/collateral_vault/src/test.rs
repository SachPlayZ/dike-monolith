#![cfg(test)]

use super::*;
use conditional_tokens::{DikeConditionalTokens, DikeConditionalTokensClient};
use dike_types::{FeeConfig, MarketData, MarketStatus};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    Env, String,
};

#[contracttype]
#[derive(Clone)]
pub enum RegKey {
    Collateral,
    Resolved,
}

#[contract]
pub struct FixedRegistry;

#[contractimpl]
impl FixedRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
        env.storage().instance().set(&RegKey::Resolved, &false);
    }

    pub fn set_resolved(env: Env, resolved: bool) {
        env.storage().instance().set(&RegKey::Resolved, &resolved);
    }

    pub fn get_final_outcome(_env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Ok(Outcome::Yes)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        let resolved: bool = env
            .storage()
            .instance()
            .get(&RegKey::Resolved)
            .unwrap_or(false);
        Ok(MarketData {
            id: market_id,
            question: String::from_str(&env, "q"),
            question_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            rules_uri: String::from_str(&env, "rules"),
            rules_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            creator: Address::generate(&env),
            collateral,
            yes_token_id: market_id * 2,
            no_token_id: market_id * 2 + 1,
            expiry: 1_000,
            status: if resolved {
                MarketStatus::Resolved
            } else {
                MarketStatus::Live
            },
            has_final_outcome: resolved,
            final_outcome: Outcome::Yes,
            pool_id: market_id,
            bond_amount: 1,
            dispute_window: 1,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: FeeConfig::default(),
        })
    }
}

#[contract]
pub struct CancelledRegistry;

#[contractimpl]
impl CancelledRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn get_final_outcome(_env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Err(DikeError::InvalidStatus)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        Ok(MarketData {
            id: market_id,
            question: String::from_str(&env, "q"),
            question_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            rules_uri: String::from_str(&env, "rules"),
            rules_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            creator: Address::generate(&env),
            collateral,
            yes_token_id: market_id * 2,
            no_token_id: market_id * 2 + 1,
            expiry: 1_000,
            status: MarketStatus::Cancelled,
            has_final_outcome: false,
            final_outcome: Outcome::Invalid,
            pool_id: market_id,
            bond_amount: 1,
            dispute_window: 1,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: FeeConfig::default(),
        })
    }
}

// Registry that returns Outcome::Invalid as the final outcome.
#[contract]
pub struct InvalidOutcomeRegistry;

#[contractimpl]
impl InvalidOutcomeRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn get_final_outcome(_env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Ok(Outcome::Invalid)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        Ok(MarketData {
            id: market_id,
            question: String::from_str(&env, "q"),
            question_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            rules_uri: String::from_str(&env, "rules"),
            rules_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            creator: Address::generate(&env),
            collateral,
            yes_token_id: market_id * 2,
            no_token_id: market_id * 2 + 1,
            expiry: 1_000,
            status: MarketStatus::Resolved,
            has_final_outcome: true,
            final_outcome: Outcome::Invalid,
            pool_id: market_id,
            bond_amount: 1,
            dispute_window: 1,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: FeeConfig::default(),
        })
    }
}

// Registry whose status/final_outcome can be flipped mid-test — needed for
// scenarios that require a market Live at credit-open time and then
// Cancelled/Resolved afterwards (FixedRegistry only toggles Live<->Resolved,
// CancelledRegistry is permanently Cancelled from construction).
#[contracttype]
#[derive(Clone)]
pub enum MutKey {
    Collateral,
    Status,
    FinalOutcome,
}

#[contract]
pub struct MutableRegistry;

#[contractimpl]
impl MutableRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&MutKey::Collateral, &collateral);
        env.storage()
            .instance()
            .set(&MutKey::Status, &MarketStatus::Live);
        env.storage()
            .instance()
            .set(&MutKey::FinalOutcome, &Outcome::Invalid);
    }

    pub fn set_status(env: Env, status: MarketStatus) {
        env.storage().instance().set(&MutKey::Status, &status);
    }

    pub fn set_final_outcome(env: Env, outcome: Outcome) {
        env.storage()
            .instance()
            .set(&MutKey::FinalOutcome, &outcome);
    }

    pub fn get_final_outcome(env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Ok(env
            .storage()
            .instance()
            .get(&MutKey::FinalOutcome)
            .unwrap_or(Outcome::Invalid))
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&MutKey::Collateral).unwrap();
        let status: MarketStatus = env
            .storage()
            .instance()
            .get(&MutKey::Status)
            .unwrap_or(MarketStatus::Live);
        Ok(MarketData {
            id: market_id,
            question: String::from_str(&env, "q"),
            question_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            rules_uri: String::from_str(&env, "rules"),
            rules_hash: soroban_sdk::BytesN::from_array(&env, &[0; 32]),
            creator: Address::generate(&env),
            collateral,
            yes_token_id: market_id * 2,
            no_token_id: market_id * 2 + 1,
            expiry: 1_000,
            status,
            has_final_outcome: status == MarketStatus::Resolved,
            final_outcome: Outcome::Invalid,
            pool_id: market_id,
            bond_amount: 1,
            dispute_window: 1,
            has_request: false,
            request_id: 0,
            created_at: env.ledger().timestamp(),
            fee_config: FeeConfig::default(),
        })
    }
}

#[test]
fn accounting_starts_empty() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);

    let acct = client.accounting(&1);
    assert_eq!(acct.total_deposited, 0);
    assert_eq!(acct.collateral_backing, 0);
}

#[test]
fn child_prediction_uses_sixty_percent_parent_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("registry"), &registry_id);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);

    assert_eq!(client.user_deposit(&1, &alice), 100);
    assert_eq!(
        client.child_avail_for_outcome(&1, &alice, &Outcome::Yes),
        60
    );

    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);

    assert_eq!(client.child_collateral_used(&1, &alice), 60);
    assert_eq!(client.child_avail_for_outcome(&1, &alice, &Outcome::Yes), 0);
    assert_eq!(client.child_parent(&2, &alice), 1);
    assert_eq!(client.child_collateral_loan(&1, &2, &alice), 60);

    let parent = client.accounting(&1);
    let child = client.accounting(&2);
    assert_eq!(parent.child_collateral_issued, 60);
    assert_eq!(child.total_deposited, 60);
    assert_eq!(child.collateral_backing, 60);
}

#[test]
fn child_prediction_cannot_exceed_limit_or_chain_again() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("registry"), &registry_id);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);

    assert_eq!(
        client.try_open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &61),
        Err(Ok(DikeError::ChildCollateralLimitExceeded))
    );

    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);

    assert_eq!(
        client.try_open_child_credit_for_trade(&alice, &2, &Outcome::Yes, &3, &Outcome::Yes, &1),
        Err(Ok(DikeError::ChainDepthExceeded))
    );
}

#[test]
fn legacy_child_funding_is_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &140);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("registry"), &registry_id);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);

    assert_eq!(
        client.try_fund_child_prediction(&alice, &1, &2, &60),
        Err(Ok(DikeError::Unauthorized))
    );
}

#[test]
fn child_win_redeem_repays_loan_before_user_profit() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let lp = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);
    stellar.mint(&lp, &100);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.deposit_for_market(&token, &lp, &2, &100);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    tokens.mint_complete_set(&alice, &2, &100);
    FixedRegistryClient::new(&env, &registry_id).set_resolved(&true);

    let before = TokenClient::new(&env, &token).balance(&alice);
    let payout = client.redeem_resolved(&token, &alice, &2, &Outcome::Yes, &100);

    assert_eq!(payout, 40);
    assert_eq!(TokenClient::new(&env, &token).balance(&alice), before + 40);
    assert_eq!(client.child_debt(&2, &alice), 0);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 0);
}

#[test]
fn parent_win_redeem_is_net_of_unpaid_child_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    tokens.mint_complete_set(&alice, &1, &100);
    FixedRegistryClient::new(&env, &registry_id).set_resolved(&true);

    let before = TokenClient::new(&env, &token).balance(&alice);
    let payout = client.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &100);

    assert_eq!(payout, 40);
    assert_eq!(TokenClient::new(&env, &token).balance(&alice), before + 40);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 0);
}

#[test]
fn child_redeem_after_parent_repayment_does_not_double_charge_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let lp = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);
    stellar.mint(&lp, &100);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.deposit_for_market(&token, &lp, &2, &100);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    tokens.mint_complete_set(&alice, &1, &100);
    tokens.mint_complete_set(&alice, &2, &100);
    FixedRegistryClient::new(&env, &registry_id).set_resolved(&true);

    let parent_payout = client.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &100);
    assert_eq!(parent_payout, 40);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 0);
    assert_eq!(client.child_debt(&2, &alice), 60);

    let child_payout = client.redeem_resolved(&token, &alice, &2, &Outcome::Yes, &100);
    assert_eq!(child_payout, 100);
    assert_eq!(client.child_debt(&2, &alice), 0);
}

#[test]
fn cancelled_market_redeems_each_side_at_half_value() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(CancelledRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &100);
    tokens.mint_complete_set(&alice, &1, &100);
    let before = TokenClient::new(&env, &token).balance(&alice);
    assert_eq!(
        client.redeem_cancelled(&token, &alice, &1, &Outcome::Yes, &100),
        50
    );
    assert_eq!(
        client.redeem_cancelled(&token, &alice, &1, &Outcome::No, &100),
        50
    );
    assert_eq!(TokenClient::new(&env, &token).balance(&alice), before + 100);
    assert_eq!(tokens.balance(&alice, &1, &Outcome::Yes), 0);
    assert_eq!(tokens.balance(&alice, &1, &Outcome::No), 0);
}

// --- Item 7: dust carry tests ---

/// With odd amounts the carry accumulator prevents any stroop from vanishing.
/// Two callers each redeem 3 tokens; naive integer division gives 1+1=2 but
/// the caller deposited 3+3=6 and expects 3 back total.  With carry: first
/// redeem of 3 → carry=3, payout=1 (carry+3=6/2=3... wait, first call:
/// carry=0, effective=3, payout=3/2=1, new_carry=3-2=1).
/// Second redeem of 3 → effective=1+3=4, payout=2, new_carry=0.
/// Total paid: 1+2=3 = 6/2 — no dust lost.
#[test]
fn odd_amount_cancelled_redeem_uses_dust_carry_no_loss() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &3);
    stellar.mint(&bob, &3);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(CancelledRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &3);
    client.deposit_for_market(&token, &bob, &1, &3);
    tokens.mint_complete_set(&alice, &1, &3);
    tokens.mint_complete_set(&bob, &1, &3);

    let alice_payout = client.redeem_cancelled(&token, &alice, &1, &Outcome::Yes, &3);
    let bob_payout = client.redeem_cancelled(&token, &bob, &1, &Outcome::No, &3);
    // Total payout must equal deposited/2 = 3 (no dust lost)
    assert_eq!(alice_payout + bob_payout, 3);
}

/// Same guarantee for redeem_resolved when final outcome is Invalid.
#[test]
fn odd_amount_invalid_resolved_redeem_uses_dust_carry_no_loss() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &3);
    stellar.mint(&bob, &3);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(InvalidOutcomeRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &3);
    client.deposit_for_market(&token, &bob, &1, &3);
    tokens.split_position(&alice, &1, &3);
    tokens.split_position(&bob, &1, &3);

    let alice_payout = client.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &3);
    let bob_payout = client.redeem_resolved(&token, &bob, &1, &Outcome::No, &3);
    assert_eq!(alice_payout + bob_payout, 3);
}

#[test]
fn failed_solvency_check_leaves_balances_and_tokens_untouched() {
    // release_trade_payout (used by normal AMM sells) is NOT part of the
    // pro-rata haircut system — an over-claim there still hard-reverts
    // atomically, unlike redeem_resolved (see the sibling test below).
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);

    let vault_id = env.register(CollateralVault, (&admin, &treasury));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let registry_id = env.register(FixedRegistry, (&token,));
    vault.set_role(&symbol_short!("amm"), &admin);
    vault.set_role(&symbol_short!("registry"), &registry_id);

    vault.deposit_for_market(&token, &alice, &1, &100);
    vault.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);

    let before_user_balance = stellar.balance(&alice);
    let before_vault_balance = stellar.balance(&vault_id);
    assert!(matches!(
        vault.try_release_trade_payout(&token, &alice, &1, &Outcome::Yes, &100, &101),
        Err(Ok(DikeError::InsufficientCollateral))
    ));
    assert_eq!(vault.root_stake(&1, &alice, &Outcome::Yes), 100);
    assert_eq!(vault.user_deposit(&1, &alice), 100);
    assert_eq!(stellar.balance(&alice), before_user_balance);
    assert_eq!(stellar.balance(&vault_id), before_vault_balance);
}

#[test]
fn redeem_resolved_overclaim_haircuts_instead_of_reverting() {
    // Gap 3 fix: an over-claim against redeem_resolved (more tokens redeemed
    // than the market's real collateral_backing can cover) used to hard-revert
    // with InsufficientCollateral, leaving whoever redeems last with nothing.
    // It now gracefully caps the payout to whatever's actually there instead.
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);

    let vault_id = env.register(CollateralVault, (&admin, &treasury));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(FixedRegistry, (&token,));
    vault.set_role(&symbol_short!("amm"), &admin);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    vault.deposit_for_market(&token, &alice, &1, &100);
    vault.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    FixedRegistryClient::new(&env, &registry_id).set_resolved(&true);

    tokens.mint_complete_set(&alice, &1, &200);
    let before_user_balance = stellar.balance(&alice);

    // Claiming 200 against a market that only really has 100 — capped, not reverted.
    let payout = vault.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &200);
    assert_eq!(payout, 100);
    assert_eq!(stellar.balance(&alice), before_user_balance + 100);
    assert_eq!(tokens.balance(&alice, &1, &Outcome::Yes), 0);
    assert_eq!(vault.accounting(&1).redeemed, 100);

    // Nothing left — a second redeemer of the same (now-exhausted) claim pool
    // gets zero rather than a revert.
    tokens.mint_complete_set(&alice, &1, &1);
    let second_payout = vault.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &1);
    assert_eq!(second_payout, 0);
}

#[test]
fn repay_child_collateral_restores_parent_backing_with_fresh_money() {
    // Previously a stub that always returned Unauthorized — no way to close a
    // credit line proactively. Voluntary repayment should top the parent's
    // collateral_backing back up using fresh money, and refund any overpay.
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &200);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let registry_id = env.register(FixedRegistry, (&token,));
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("registry"), &registry_id);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    assert_eq!(client.accounting(&1).collateral_backing, 40);

    // Overpay by 20 — only 60 is owed, the rest should come straight back.
    let before = stellar.balance(&alice);
    let applied = client.repay_child_collateral(&token, &alice, &2, &80);
    assert_eq!(applied, 60);
    assert_eq!(stellar.balance(&alice), before - 60);
    assert_eq!(client.accounting(&1).collateral_backing, 100);
    assert_eq!(client.accounting(&1).child_collateral_repaid, 60);
    assert_eq!(client.child_debt(&2, &alice), 0);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 0);
    assert_eq!(
        client.child_avail_for_outcome(&1, &alice, &Outcome::Yes),
        60
    );
}

#[test]
fn redeem_cancelled_withholds_against_outstanding_child_debt() {
    // Gap 4: a cancelled parent that's still backing live child credit must
    // not pay out a full refund while the debt stays marked healthy against
    // collateral that's already left the building.
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(MutableRegistry, (&token,));
    let registry = MutableRegistryClient::new(&env, &registry_id);
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    tokens.mint_complete_set(&alice, &1, &100);

    registry.set_status(&MarketStatus::Cancelled);

    let before = stellar.balance(&alice);
    // Debt is scoped to the YES side (that's what backed the credit draw) —
    // her 100-token YES cancellation refund is only 50 (invalid_refund
    // halves it), which doesn't even fully cover the 60 debt: it's withheld
    // entirely, and 10 stays outstanding. Old code paid this out in full and
    // never touched the debt at all — this is the actual regression test for
    // the audit's "cancel-and-drain" exploit.
    let yes_payout = client.redeem_cancelled(&token, &alice, &1, &Outcome::Yes, &100);
    assert_eq!(yes_payout, 0);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 10);

    // Her NO side is a separate, unencumbered position (debt was only ever
    // opened against her YES stake) — its refund is untouched by the debt.
    let no_payout = client.redeem_cancelled(&token, &alice, &1, &Outcome::No, &100);
    assert_eq!(no_payout, 50);
    assert_eq!(stellar.balance(&alice), before + 50);
}

#[test]
fn parent_default_haircuts_other_stakers_after_insurance_partially_covers() {
    // Gap 3, full pro-rata haircut: alice borrows against her own YES stake
    // and YES loses — her debt can't come out of her own (zero) payout, so
    // it becomes a default. Bob staked the WINNING NO side and never
    // borrowed anything, but the same market pool backs both sides — once
    // insurance partially covers alice's default and the rest becomes a
    // recorded shortfall, bob's own healthy winning claim gets pro-rata
    // haircut too, instead of either paying him in full (ignoring the
    // deficit) or hard-reverting (the old, worse failure mode).
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&alice, &100);
    stellar.mint(&bob, &50);

    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(MutableRegistry, (&token,));
    let registry = MutableRegistryClient::new(&env, &registry_id);
    client.set_role(&symbol_short!("amm"), &admin);
    client.set_role(&symbol_short!("tokens"), &tokens_id);
    client.set_role(&symbol_short!("registry"), &registry_id);
    client.set_role(&symbol_short!("gov"), &admin);
    tokens.set_role(&symbol_short!("vault"), &id);
    tokens.set_role(&symbol_short!("amm"), &admin);

    // Alice stakes YES (and borrows against it), bob stakes NO — same pool.
    client.deposit_for_market(&token, &alice, &1, &100);
    client.record_cash_stake(&alice, &1, &Outcome::Yes, &100, &100);
    client.deposit_for_market(&token, &bob, &1, &50);
    client.record_cash_stake(&bob, &1, &Outcome::No, &50, &50);
    client.open_child_credit_for_trade(&alice, &1, &Outcome::Yes, &2, &Outcome::Yes, &60);
    assert_eq!(client.accounting(&1).collateral_backing, 90); // 150 - 60

    // Seed a partial insurance reserve via a fee sweep so the default
    // doesn't fall entirely on shortfall. Fee must fit within the market's
    // real 90 backing.
    client.collect_fee(&1, &0, &50, &0);
    client.sweep_protocol_fees(&token, &1);
    assert_eq!(client.insurance_reserve(), 10); // 20% of the 50 swept
    assert_eq!(client.accounting(&1).collateral_backing, 40); // 90 - 50 fee

    tokens.mint_complete_set(&alice, &1, &100);
    tokens.mint_complete_set(&bob, &1, &50);

    // Market resolves NO — alice's YES stake (and her debt) is a total loss;
    // bob's NO stake wins.
    registry.set_status(&MarketStatus::Resolved);
    registry.set_final_outcome(&Outcome::No);

    let alice_payout = client.redeem_resolved(&token, &alice, &1, &Outcome::Yes, &100);
    assert_eq!(alice_payout, 0);
    assert_eq!(client.parent_debt(&1, &alice, &Outcome::Yes), 0); // written off, not left dangling
    assert_eq!(client.insurance_reserve(), 0); // fully drawn down
    let accounting = client.accounting(&1);
    assert_eq!(accounting.collateral_backing, 40 + 10); // insurance top-up = 50
    assert_eq!(accounting.shortfall, 50); // 60 defaulted - 10 insurance-covered

    // Bob's full claim is 50; haircut ratio = collateral_backing / (collateral_backing + shortfall)
    // = 50 / 100 → 50 * 50 / 100 = 25, not the full 50 and not zero.
    let bob_payout = client.redeem_resolved(&token, &bob, &1, &Outcome::No, &50);
    assert_eq!(bob_payout, 25);
    assert_eq!(stellar.balance(&bob), 25);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let token = Address::generate(&env);
    let user = Address::generate(&env);

    // gov role not configured
    assert!(matches!(
        client.try_set_treasury(&Address::generate(&env)),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_pause(&true),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_sweep_protocol_fees(&token, &1),
        Err(Ok(DikeError::Unauthorized))
    ));

    // amm role not configured
    assert!(matches!(
        client.try_deposit_for_market(&token, &user, &1, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_record_cash_stake(&user, &1, &Outcome::Yes, &100, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_open_child_credit_for_trade(&user, &1, &Outcome::Yes, &2, &Outcome::Yes, &1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_release_trade_payout(&token, &user, &1, &Outcome::Yes, &1, &1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_collect_fee(&1, &0, &0, &0),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_claim_lp_fees(&token, &1, &user, &1),
        Err(Ok(DikeError::Unauthorized))
    ));

    // tokens role not configured
    assert!(matches!(
        client.try_assert_position_transfer_allowed(&user, &1, &Outcome::Yes, &1),
        Err(Ok(DikeError::Unauthorized))
    ));

    // oracle role not configured
    assert!(matches!(
        client.try_release_bond(&token, &user, &1, &1, &false),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_slash_bond(&token, &user, &1, &1, &false, &Address::generate(&env)),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_role(&symbol_short!("gov"), &other).is_err());
}

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
}

#[contract]
pub struct FixedRegistry;

#[contractimpl]
impl FixedRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn get_final_outcome(_env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Ok(Outcome::Yes)
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
fn legacy_child_funding_and_repayment_are_disabled() {
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
    assert_eq!(
        client.try_repay_child_collateral(&token, &alice, &1, &2, &40),
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

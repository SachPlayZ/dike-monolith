#![cfg(test)]

use super::*;
use amm::{DikeAMM, DikeAMMClient};
use cod_oracle::{CODOracle, CODOracleClient};
use collateral_vault::{CollateralVault, CollateralVaultClient};
use conditional_tokens::{DikeConditionalTokens, DikeConditionalTokensClient};
use council_of_dike::{CouncilOfDike, CouncilOfDikeClient};
use dike_types::{FeeConfig, MarketStatus, Outcome};
use fee_manager::FeeManager;
use market_registry::{DikeMarketRegistry, DikeMarketRegistryClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    BytesN, Env, String,
};

fn cfg(env: &Env, creator: &Address, collateral: &Address) -> MarketConfig {
    MarketConfig {
        question: String::from_str(env, "Will Dike launch?"),
        question_hash: BytesN::from_array(env, &[1; 32]),
        rules_uri: String::from_str(env, "ipfs://rules"),
        rules_hash: BytesN::from_array(env, &[2; 32]),
        expiry: 10_000,
        collateral: collateral.clone(),
        bond_amount: 500,
        dispute_window: 300,
        category: String::from_str(env, "crypto"),
        creator: creator.clone(),
        fee_config: FeeConfig::default(),
    }
}

#[test]
fn full_production_graph_trades_resolves_and_redeems_with_real_asset_contract() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let treasury = Address::generate(&env);
    let creator = Address::generate(&env);
    let trader = Address::generate(&env);
    let proposer = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let collateral = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let asset = StellarAssetClient::new(&env, &collateral);
    asset.mint(&creator, &5_000);
    asset.mint(&trader, &5_000);
    asset.mint(&proposer, &1_000);

    let registry_id = env.register(DikeMarketRegistry, (&admin,));
    let registry = DikeMarketRegistryClient::new(&env, &registry_id);
    let vault_id = env.register(CollateralVault, (&admin, &treasury));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let amm_id = env.register(DikeAMM, (&admin,));
    let amm = DikeAMMClient::new(&env, &amm_id);
    let fee_manager_id = env.register(FeeManager, (&admin, &gov, &500i128, &100u32));
    let oracle_id = env.register(CODOracle, (&admin,));
    let oracle = CODOracleClient::new(&env, &oracle_id);
    let council_id = env.register(CouncilOfDike, (&admin,));
    let council = CouncilOfDikeClient::new(&env, &council_id);
    let factory_id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let factory = DikeMarketFactoryClient::new(&env, &factory_id);

    registry.set_role(&symbol_short!("factory"), &factory_id);
    registry.set_role(&symbol_short!("oracle"), &oracle_id);
    registry.set_role(&symbol_short!("gov"), &gov);
    registry.set_supported_collateral(&collateral, &true);

    tokens.set_role(&symbol_short!("vault"), &vault_id);
    tokens.set_role(&symbol_short!("amm"), &amm_id);

    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("oracle"), &oracle_id);
    vault.set_role(&symbol_short!("amm"), &amm_id);
    vault.set_role(&symbol_short!("gov"), &gov);
    vault.set_role(&symbol_short!("registry"), &registry_id);

    amm.set_role(&symbol_short!("factory"), &factory_id);
    amm.set_role(&symbol_short!("gov"), &gov);
    amm.set_modules(&vault_id, &tokens_id, &collateral, &registry_id);

    oracle.set_role(&symbol_short!("gov"), &gov);
    oracle.set_role(&symbol_short!("council"), &council_id);
    oracle.set_role(&symbol_short!("registry"), &registry_id);
    oracle.set_role(&symbol_short!("vault"), &vault_id);

    council.set_role(&symbol_short!("gov"), &gov);
    council.set_role(&symbol_short!("oracle"), &oracle_id);

    factory.set_modules(
        &registry_id,
        &tokens_id,
        &vault_id,
        &amm_id,
        &fee_manager_id,
    );
    factory.set_creator(&creator, &true);
    factory.set_collateral(&collateral, &true);

    let config = cfg(&env, &creator, &collateral);
    let market = factory.create_market(&config, &1_000, &5_000);
    assert_eq!(market.status, MarketStatus::Live);
    assert_eq!(
        TokenClient::new(&env, &collateral).balance(&vault_id),
        1_000
    );

    let bought_yes = amm.buy_yes(&trader, &market.pool_id, &500, &1, &10_000);
    assert!(bought_yes > 0);
    assert_eq!(
        tokens.balance(&trader, &market.id, &Outcome::Yes),
        bought_yes
    );

    env.ledger().set_timestamp(config.expiry + 1);
    let request_id = oracle.request_resolution(
        &market.id,
        &config.question_hash,
        &config.rules_uri,
        &config.expiry,
        &config.bond_amount,
        &config.dispute_window,
    );
    oracle.propose_outcome(
        &proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&env, "ipfs://evidence"),
    );
    env.ledger()
        .set_timestamp(config.expiry + config.dispute_window + 2);
    assert_eq!(oracle.finalize_undisputed(&request_id), Outcome::Yes);
    assert_eq!(registry.get_final_outcome(&market.id), Outcome::Yes);

    let trader_before = TokenClient::new(&env, &collateral).balance(&trader);
    let payout =
        vault.redeem_resolved(&collateral, &trader, &market.id, &Outcome::Yes, &bought_yes);
    assert_eq!(payout, bought_yes);
    assert_eq!(
        TokenClient::new(&env, &collateral).balance(&trader),
        trader_before + payout
    );
}

#[test]
fn creates_curated_market() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let collateral = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    StellarAssetClient::new(&env, &collateral).mint(&creator, &2_000);
    let registry_id = env.register(DikeMarketRegistry, (&admin,));
    let registry = DikeMarketRegistryClient::new(&env, &registry_id);
    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let amm_id = env.register(DikeAMM, (&admin,));
    let amm = DikeAMMClient::new(&env, &amm_id);
    let fee_manager_id = env.register(FeeManager, (&admin, &gov, &500i128, &100u32));
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let client = DikeMarketFactoryClient::new(&env, &id);
    registry.set_role(&symbol_short!("factory"), &id);
    registry.set_supported_collateral(&collateral, &true);
    vault.set_role(&symbol_short!("amm"), &amm_id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &amm_id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    amm.set_role(&symbol_short!("factory"), &id);
    amm.set_modules(&vault_id, &tokens_id, &collateral, &registry_id);
    client.set_modules(
        &registry_id,
        &tokens_id,
        &vault_id,
        &amm_id,
        &fee_manager_id,
    );
    client.set_creator(&creator, &true);
    client.set_collateral(&collateral, &true);

    let market = client.create_market(&cfg(&env, &creator, &collateral), &1_000, &5_000);
    assert_eq!(market.id, 1);
    assert_eq!(market.yes_token_id, 2);
    assert_eq!(market.no_token_id, 3);
    assert_eq!(registry.get_status(&1), MarketStatus::Live);
    assert_eq!(amm.pool(&1).market_id, 1);
    assert_eq!(
        TokenClient::new(&env, &collateral).balance(&vault_id),
        1_000
    );

    assert!(matches!(
        client.try_create_market(&cfg(&env, &creator, &collateral), &1_000, &6_000),
        Err(Ok(DikeError::InvalidInput))
    ));

    let mut bad_cfg = cfg(&env, &creator, &collateral);
    bad_cfg.fee_config.lp_fee_share_bps = 9_000;
    assert!(matches!(
        client.try_create_market(&bad_cfg, &100, &5_000),
        Err(Ok(DikeError::InvalidInput))
    ));
}

#[test]
fn rejects_overflowing_expiry_floor() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(10);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let creator = Address::generate(&env);
    let collateral = Address::generate(&env);
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &u64::MAX));
    let client = DikeMarketFactoryClient::new(&env, &id);

    assert!(matches!(
        client.try_create_market(&cfg(&env, &creator, &collateral), &1_000, &5_000),
        Err(Ok(DikeError::ArithmeticError))
    ));
}

// --- Item 3: counter-divergence safety under failed create_market ---

/// Verifies that a failed create_market call (registry rejects because the
/// collateral is not supported there, even though factory approved it) leaves
/// the factory's NextMarketId counter unchanged.  This exercises the atomicity
/// guarantee documented in create_market's doc comment.
#[test]
fn failed_create_market_leaves_counter_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let collateral = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    soroban_sdk::token::StellarAssetClient::new(&env, &collateral).mint(&creator, &5_000);

    // Registry: set factory role but do NOT approve the collateral in registry.
    let registry_id = env.register(DikeMarketRegistry, (&admin,));
    let registry = DikeMarketRegistryClient::new(&env, &registry_id);
    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let amm_id = env.register(DikeAMM, (&admin,));
    let amm = DikeAMMClient::new(&env, &amm_id);
    let fee_manager_id = env.register(FeeManager, (&admin, &gov, &500i128, &100u32));
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let client = DikeMarketFactoryClient::new(&env, &id);

    registry.set_role(&symbol_short!("factory"), &id);
    // Deliberately NOT calling registry.set_supported_collateral — so registry rejects.
    vault.set_role(&symbol_short!("amm"), &amm_id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &amm_id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    amm.set_role(&symbol_short!("factory"), &id);
    amm.set_modules(&vault_id, &tokens_id, &collateral, &registry_id);
    client.set_modules(
        &registry_id,
        &tokens_id,
        &vault_id,
        &amm_id,
        &fee_manager_id,
    );
    // Creator and collateral approved in FACTORY but not in REGISTRY.
    client.set_creator(&creator, &true);
    client.set_collateral(&collateral, &true);

    let counter_before = client.next_market_id();
    assert_eq!(counter_before, 1);

    // create_market: factory validate passes, then registry.register_market
    // fails (UnsupportedCollateral in registry) → tx reverts → counter unchanged.
    assert!(client
        .try_create_market(&cfg(&env, &creator, &collateral), &1_000, &5_000)
        .is_err());

    assert_eq!(client.next_market_id(), counter_before);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let client = DikeMarketFactoryClient::new(&env, &id);
    let creator = Address::generate(&env);
    let collateral = Address::generate(&env);
    // governance address in storage but not as a role — require_governance reads DataKey::Governance
    // which IS set in the constructor to `gov`.  To test rejection, use a different caller by not
    // calling set_creator/set_collateral as `gov`.
    // For set_creator/set_collateral the fn calls require_governance which requires gov.require_auth().
    // Without mock_all_auths on gov, this panics.  With mock_all_auths (env above), require_auth
    // passes for any address — so these calls succeed.  The role-rejection pattern for this contract
    // is via require_timelock (which reads governance.timelock).  Test that:
    // set_creator_by_timelock requires require_timelock which calls governance.timelock() cross-contract.
    // Governance is set to `gov` (plain address, no contract), so the cross-contract call panics.
    assert!(client.try_set_creator_by_timelock(&creator, &true).is_err());
}

#[test]
fn governance_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let client = DikeMarketFactoryClient::new(&env, &id);
    let other = Address::generate(&env);
    // set_admin needs admin auth
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_governance(&other).is_err());
    assert!(client
        .try_set_modules(&other, &other, &other, &other, &other)
        .is_err());
    // set_creator needs governance auth (gov.require_auth() panics)
    assert!(client.try_set_creator(&other, &true).is_err());
    assert!(client.try_set_collateral(&other, &true).is_err());
    assert!(client.try_pause(&true).is_err());
}

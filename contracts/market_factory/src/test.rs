#![cfg(test)]

use super::*;
use amm::{DikeAMM, DikeAMMClient};
use cod_oracle::{CODOracle, CODOracleClient};
use collateral_vault::{CollateralVault, CollateralVaultClient};
use conditional_tokens::{DikeConditionalTokens, DikeConditionalTokensClient};
use council_of_dike::{CouncilOfDike, CouncilOfDikeClient};
use dike_types::{FeeConfig, Outcome};
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
        &Address::generate(&env),
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

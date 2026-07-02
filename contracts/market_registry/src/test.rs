#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    BytesN, Env, String,
};

fn cfg(env: &Env, creator: &Address, collateral: &Address) -> MarketConfig {
    MarketConfig {
        question: String::from_str(env, "Will Dike ship on Stellar?"),
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
fn stores_rules_and_blocks_double_resolution() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let oracle = Address::generate(&env);
    let gov = Address::generate(&env);
    let collateral = Address::generate(&env);
    let creator = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_role(&symbol_short!("gov"), &gov);
    client.set_supported_collateral(&collateral, &true);

    let market_id = client.register_market(&cfg(&env, &creator, &collateral), &1, &2, &1);
    let market = client.get_market(&market_id);
    assert_eq!(market.question_hash, BytesN::from_array(&env, &[1; 32]));
    assert_eq!(market.rules_hash, BytesN::from_array(&env, &[2; 32]));

    client.activate_market(&market_id);
    env.ledger().set_timestamp(10_001);
    client.close_trading(&market_id);
    client.mark_resolution_requested(&market_id, &7);
    client.mark_proposed(&market_id);
    client.set_final_outcome(&market_id, &Outcome::Yes);
    assert!(client
        .try_set_final_outcome(&market_id, &Outcome::No)
        .is_err());

    let mut bad_cfg = cfg(&env, &creator, &collateral);
    bad_cfg.fee_config.trading_fee_bps = 1_001;
    assert!(matches!(
        client.try_register_market(&bad_cfg, &3, &4, &2),
        Err(Ok(DikeError::InvalidInput))
    ));
    let bad_update = FeeConfig {
        creation_fee: -1,
        ..Default::default()
    };
    assert!(matches!(
        client.try_set_fee_config(&market_id, &bad_update),
        Err(Ok(DikeError::InvalidAmount))
    ));
}

#[test]
fn generic_status_cannot_resolve_without_final_outcome() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let oracle = Address::generate(&env);
    let gov = Address::generate(&env);
    let collateral = Address::generate(&env);
    let creator = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_role(&symbol_short!("gov"), &gov);
    client.set_supported_collateral(&collateral, &true);

    let market_id = client.register_market(&cfg(&env, &creator, &collateral), &1, &2, &1);
    client.activate_market(&market_id);
    env.ledger().set_timestamp(10_001);
    client.close_trading(&market_id);
    client.mark_resolution_requested(&market_id, &7);
    client.mark_proposed(&market_id);

    assert!(matches!(
        client.try_set_status(&market_id, &MarketStatus::Resolved),
        Err(Ok(DikeError::InvalidTransition))
    ));
    assert!(matches!(
        client.try_get_final_outcome(&market_id),
        Err(Ok(DikeError::InvalidStatus))
    ));

    client.set_final_outcome(&market_id, &Outcome::Yes);
    let market = client.get_market(&market_id);
    assert_eq!(market.status, MarketStatus::Resolved);
    assert!(market.has_final_outcome);
    assert_eq!(client.get_final_outcome(&market_id), Outcome::Yes);
}

#[test]
fn paused_markets_can_be_emergency_closed() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let oracle = Address::generate(&env);
    let gov = Address::generate(&env);
    let collateral = Address::generate(&env);
    let creator = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_role(&symbol_short!("gov"), &gov);
    client.set_supported_collateral(&collateral, &true);

    let market_id = client.register_market(&cfg(&env, &creator, &collateral), &1, &2, &1);
    client.activate_market(&market_id);
    client.set_status(&market_id, &MarketStatus::Paused);
    env.ledger().set_timestamp(10_001);
    client.close_trading(&market_id);

    assert_eq!(client.get_status(&market_id), MarketStatus::TradingClosed);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let collateral = Address::generate(&env);
    let creator = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    // factory role not configured
    assert!(matches!(
        client.try_register_market(&cfg(&env, &creator, &collateral), &1, &2, &1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_activate_market(&1),
        Err(Ok(DikeError::Unauthorized))
    ));
    // gov role not configured
    assert!(matches!(
        client.try_set_fee_config(&1, &FeeConfig::default()),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_set_status(&1, &MarketStatus::Live),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_cancel_market(&1),
        Err(Ok(DikeError::Unauthorized))
    ));
    // oracle role not configured
    assert!(matches!(
        client.try_mark_resolution_requested(&1, &1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_mark_proposed(&1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_mark_disputed(&1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_mark_council_voting(&1),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_set_final_outcome(&1, &Outcome::Yes),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client
        .try_set_role(&symbol_short!("factory"), &other)
        .is_err());
    assert!(client.try_set_supported_collateral(&other, &true).is_err());
    assert!(client.try_pause_system(&true).is_err());
}

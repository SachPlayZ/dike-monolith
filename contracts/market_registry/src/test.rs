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
    let collateral = Address::generate(&env);
    let creator = Address::generate(&env);
    let id = env.register(DikeMarketRegistry, (&admin,));
    let client = DikeMarketRegistryClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    client.set_role(&symbol_short!("oracle"), &oracle);
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
}

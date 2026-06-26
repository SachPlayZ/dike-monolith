#![cfg(test)]

use super::*;
use dike_types::FeeConfig;
use soroban_sdk::{testutils::Address as _, BytesN, Env, String};

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
fn creates_curated_market() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let creator = Address::generate(&env);
    let collateral = Address::generate(&env);
    let id = env.register(DikeMarketFactory, (&admin, &gov, &100i128, &60u64));
    let client = DikeMarketFactoryClient::new(&env, &id);
    client.set_creator(&creator, &true);
    client.set_collateral(&collateral, &true);

    let market = client.create_market(&cfg(&env, &creator, &collateral), &1_000, &5_000);
    assert_eq!(market.id, 1);
    assert_eq!(market.yes_token_id, 2);
    assert_eq!(market.no_token_id, 3);
}

#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn seed_and_trade_updates_reserves() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    let out = client.buy_yes(&trader, &pool_id, &1_000, &1, &100);
    assert!(out > 0);
    let pool = client.pool(&pool_id);
    assert!(pool.yes_reserve < 10_000);
    assert!(pool.no_reserve > 10_000);
    assert!(pool.accumulated_lp_fees > 0);
}

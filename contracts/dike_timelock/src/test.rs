#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    BytesN, Env,
};

#[test]
fn queues_and_executes_after_delay() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);
    let target = Address::generate(&env);
    let id = env.register(
        DikeTimelock,
        (&admin, &proposer, &executor, &10u64, &100u64),
    );
    let client = DikeTimelockClient::new(&env, &id);
    let hash = BytesN::from_array(&env, &[9; 32]);
    let action_id = client.queue(&TimelockActionKind::FeeConfig, &target, &hash, &10);
    assert!(client.try_execute(&action_id, &hash).is_err());
    env.ledger().set_timestamp(111);
    assert!(client.execute(&action_id, &hash).executed);
}

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    BytesN, Env, String,
};

#[test]
fn finalizes_undisputed_after_window() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let id = env.register(CODOracle, (&admin,));
    let client = CODOracleClient::new(&env, &id);

    let request_id = client.request_resolution(
        &1,
        &BytesN::from_array(&env, &[1; 32]),
        &String::from_str(&env, "ipfs://rules"),
        &999,
        &500,
        &100,
    );
    client.propose_outcome(
        &proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&env, "ipfs://evidence"),
    );
    env.ledger().set_timestamp(1_101);
    assert_eq!(client.finalize_undisputed(&request_id), Outcome::Yes);
    assert!(client.try_finalize_undisputed(&request_id).is_err());
}

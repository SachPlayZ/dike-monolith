#![cfg(test)]

use super::*;
use dike_types::OpenCaseConfig;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    BytesN, Env, String,
};

#[test]
fn commit_reveal_and_finalize() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let oracle = Address::generate(&env);
    let voter = Address::generate(&env);
    let proposer = Address::generate(&env);
    let disputer = Address::generate(&env);
    let id = env.register(CouncilOfDike, (&admin,));
    let client = CouncilOfDikeClient::new(&env, &id);
    client.set_role(&symbol_short!("gov"), &gov);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_member(&voter, &true);

    let case_id = client.open_case(
        &1,
        &1,
        &proposer,
        &Outcome::Yes,
        &String::from_str(&env, "ipfs://p"),
        &disputer,
        &Outcome::No,
        &String::from_str(&env, "ipfs://d"),
        &OpenCaseConfig {
            proposal_bond: 500,
            dispute_bond: 500,
            commit_duration: 10,
            reveal_duration: 10,
        },
    );
    let salt = BytesN::from_array(&env, &[7; 32]);
    let commitment = client.vote_commitment(&case_id, &voter, &Outcome::No, &salt);
    client.commit_vote(&voter, &case_id, &commitment);
    env.ledger().set_timestamp(12);
    assert!(client
        .try_reveal_vote(&voter, &case_id, &Outcome::Yes, &salt)
        .is_err());
    client.reveal_vote(&voter, &case_id, &Outcome::No, &salt);
    env.ledger().set_timestamp(25);
    assert_eq!(client.finalize_case(&case_id), Outcome::No);
    assert!(client.claim_reward(&voter, &case_id));
}

#[test]
fn pause_blocks_case_mutations_and_zero_windows_are_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let oracle = Address::generate(&env);
    let voter = Address::generate(&env);
    let proposer = Address::generate(&env);
    let disputer = Address::generate(&env);
    let id = env.register(CouncilOfDike, (&admin,));
    let client = CouncilOfDikeClient::new(&env, &id);
    client.set_role(&symbol_short!("gov"), &gov);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_member(&voter, &true);

    assert!(client
        .try_open_case(
            &1,
            &1,
            &proposer,
            &Outcome::Yes,
            &String::from_str(&env, "ipfs://p"),
            &disputer,
            &Outcome::No,
            &String::from_str(&env, "ipfs://d"),
            &OpenCaseConfig {
                proposal_bond: 500,
                dispute_bond: 500,
                commit_duration: 0,
                reveal_duration: 10,
            },
        )
        .is_err());

    let case_id = client.open_case(
        &1,
        &1,
        &proposer,
        &Outcome::Yes,
        &String::from_str(&env, "ipfs://p"),
        &disputer,
        &Outcome::No,
        &String::from_str(&env, "ipfs://d"),
        &OpenCaseConfig {
            proposal_bond: 500,
            dispute_bond: 500,
            commit_duration: 10,
            reveal_duration: 10,
        },
    );
    let salt = BytesN::from_array(&env, &[9; 32]);
    let commitment = client.vote_commitment(&case_id, &voter, &Outcome::Yes, &salt);
    client.pause(&true);
    assert!(client
        .try_commit_vote(&voter, &case_id, &commitment)
        .is_err());
    assert!(client
        .try_reveal_vote(&voter, &case_id, &Outcome::Yes, &salt)
        .is_err());
    env.ledger().set_timestamp(25);
    assert!(client.try_finalize_case(&case_id).is_err());
}

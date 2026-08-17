#![cfg(test)]

use super::*;
use dike_types::OpenCaseConfig;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
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
            token: Address::generate(&env),
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
    assert!(client.claim_reward(&voter, &case_id).0);
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
                token: Address::generate(&env),
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
            token: Address::generate(&env),
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
    // Pause blocks new participation (commit/reveal) but not completion of
    // an already in-flight case, so it can't strand votes/bonds.
    assert!(client.try_finalize_case(&case_id).is_ok());
}

#[test]
fn governance_can_sweep_reward_pool_when_no_vote_matches_final_outcome() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let gov = Address::generate(&env);
    let oracle = Address::generate(&env);
    let yes_voter = Address::generate(&env);
    let no_voter = Address::generate(&env);
    let proposer = Address::generate(&env);
    let disputer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    let id = env.register(CouncilOfDike, (&admin,));
    let client = CouncilOfDikeClient::new(&env, &id);

    client.set_role(&symbol_short!("gov"), &gov);
    client.set_role(&symbol_short!("oracle"), &oracle);
    client.set_member(&yes_voter, &true);
    client.set_member(&no_voter, &true);

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
            token: token.clone(),
        },
    );
    let yes_salt = BytesN::from_array(&env, &[5; 32]);
    let yes_commitment = client.vote_commitment(&case_id, &yes_voter, &Outcome::Yes, &yes_salt);
    client.commit_vote(&yes_voter, &case_id, &yes_commitment);
    let no_salt = BytesN::from_array(&env, &[6; 32]);
    let no_commitment = client.vote_commitment(&case_id, &no_voter, &Outcome::No, &no_salt);
    client.commit_vote(&no_voter, &case_id, &no_commitment);
    env.ledger().set_timestamp(12);
    client.reveal_vote(&yes_voter, &case_id, &Outcome::Yes, &yes_salt);
    client.reveal_vote(&no_voter, &case_id, &Outcome::No, &no_salt);
    env.ledger().set_timestamp(25);
    assert_eq!(client.finalize_case(&case_id), Outcome::Invalid);

    stellar.mint(&id, &150);
    client.record_case_reward(&case_id, &150);
    assert_eq!(client.case_reward_pool(&case_id), 150);

    let swept = client.sweep_case_reward(&case_id, &recipient);
    assert_eq!(swept, 150);
    assert_eq!(client.case_reward_pool(&case_id), 0);
    assert_eq!(TokenClient::new(&env, &token).balance(&recipient), 150);
    assert!(client.try_claim_reward(&yes_voter, &case_id).is_ok());
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let id = env.register(CouncilOfDike, (&admin,));
    let client = CouncilOfDikeClient::new(&env, &id);
    // gov role not configured
    assert!(matches!(
        client.try_set_member(&Address::generate(&env), &true),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_pause(&true),
        Err(Ok(DikeError::Unauthorized))
    ));
    assert!(matches!(
        client.try_sweep_case_reward(&1, &Address::generate(&env)),
        Err(Ok(DikeError::Unauthorized))
    ));
    // oracle role not configured
    assert!(matches!(
        client.try_record_case_reward(&1, &100),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let id = env.register(CouncilOfDike, (&admin,));
    let client = CouncilOfDikeClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_role(&symbol_short!("gov"), &other).is_err());
}

#![cfg(test)]

use super::*;
use collateral_vault::{CollateralVault, CollateralVaultClient};
use council_of_dike::{CouncilOfDike, CouncilOfDikeClient};
use dike_types::{FeeConfig, MarketConfig, MarketStatus, OracleStatus};
use fee_manager::FeeManager;
use market_registry::{DikeMarketRegistry, DikeMarketRegistryClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    BytesN, Env, String,
};

struct Harness {
    env: Env,
    token: Address,
    proposer: Address,
    disputer: Address,
    oracle_id: Address,
    registry_id: Address,
    vault_id: Address,
    council_id: Address,
}

fn market_config(env: &Env, creator: Address, token: Address, dispute_window: u64) -> MarketConfig {
    MarketConfig {
        question: String::from_str(env, "Will this resolve?"),
        question_hash: BytesN::from_array(env, &[1; 32]),
        rules_uri: String::from_str(env, "ipfs://rules"),
        rules_hash: BytesN::from_array(env, &[2; 32]),
        expiry: 999,
        collateral: token,
        bond_amount: 500,
        dispute_window,
        category: String::from_str(env, "test"),
        creator,
        fee_config: FeeConfig::default(),
    }
}

fn setup_with_dispute_window(dispute_window: u64) -> Harness {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let disputer = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&proposer, &1_000);
    stellar.mint(&disputer, &1_000);

    let registry_id = env.register(DikeMarketRegistry, (&admin,));
    let registry = DikeMarketRegistryClient::new(&env, &registry_id);
    let vault_id = env.register(CollateralVault, (&admin, &treasury));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let council_id = env.register(CouncilOfDike, (&admin,));
    let council = CouncilOfDikeClient::new(&env, &council_id);
    let oracle_id = env.register(CODOracle, (&admin,));
    let oracle = CODOracleClient::new(&env, &oracle_id);

    registry.set_role(&symbol_short!("factory"), &factory);
    registry.set_role(&symbol_short!("oracle"), &oracle_id);
    registry.set_supported_collateral(&token, &true);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    vault.set_role(&symbol_short!("oracle"), &oracle_id);
    oracle.set_role(&symbol_short!("registry"), &registry_id);
    oracle.set_role(&symbol_short!("vault"), &vault_id);
    oracle.set_role(&symbol_short!("council"), &council_id);
    council.set_role(&symbol_short!("oracle"), &oracle_id);

    let config = market_config(&env, creator, token.clone(), dispute_window);
    let market_id = registry.register_market(&config, &2, &3, &1);
    assert_eq!(market_id, 1);
    registry.activate_market(&market_id);
    env.ledger().set_timestamp(1_000);

    Harness {
        env,
        token,
        proposer,
        disputer,
        oracle_id,
        registry_id,
        vault_id,
        council_id,
    }
}

fn setup() -> Harness {
    setup_with_dispute_window(100)
}

#[test]
fn finalizes_undisputed_end_to_end_and_releases_bond() {
    let h = setup();
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let registry = DikeMarketRegistryClient::new(&h.env, &h.registry_id);
    let vault = CollateralVaultClient::new(&h.env, &h.vault_id);
    let start_balance = TokenClient::new(&h.env, &h.token).balance(&h.proposer);
    let request_id = oracle.request_resolution(
        &1,
        &BytesN::from_array(&h.env, &[1; 32]),
        &String::from_str(&h.env, "ipfs://rules"),
        &999,
        &500,
        &100,
    );
    assert_eq!(registry.get_status(&1), MarketStatus::ResolutionRequested);

    oracle.propose_outcome(
        &h.proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&h.env, "ipfs://evidence"),
    );
    assert_eq!(registry.get_status(&1), MarketStatus::Proposed);
    assert_eq!(vault.accounting(&1).proposal_bonds, 500);
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.proposer),
        start_balance - 500
    );

    h.env.ledger().set_timestamp(1_101);
    assert_eq!(oracle.finalize_undisputed(&request_id), Outcome::Yes);
    assert_eq!(registry.get_status(&1), MarketStatus::Resolved);
    assert_eq!(registry.get_final_outcome(&1), Outcome::Yes);
    assert_eq!(vault.accounting(&1).proposal_bonds, 0);
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.proposer),
        start_balance
    );
    assert!(oracle.try_finalize_undisputed(&request_id).is_err());
}

#[test]
fn oversized_dispute_window_rejected_before_bond_lock() {
    let h = setup_with_dispute_window(u64::MAX);
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let vault = CollateralVaultClient::new(&h.env, &h.vault_id);
    let request_id = oracle.request_resolution(
        &1,
        &BytesN::from_array(&h.env, &[1; 32]),
        &String::from_str(&h.env, "ipfs://rules"),
        &999,
        &500,
        &u64::MAX,
    );

    assert!(matches!(
        oracle.try_propose_outcome(
            &h.proposer,
            &request_id,
            &Outcome::Yes,
            &String::from_str(&h.env, "ipfs://evidence"),
        ),
        Err(Ok(DikeError::ArithmeticError))
    ));
    assert_eq!(vault.accounting(&1).proposal_bonds, 0);
}

#[test]
fn invalid_request_input_does_not_close_trading() {
    let h = setup();
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let registry = DikeMarketRegistryClient::new(&h.env, &h.registry_id);

    assert!(matches!(
        oracle.try_request_resolution(
            &1,
            &BytesN::from_array(&h.env, &[1; 32]),
            &String::from_str(&h.env, ""),
            &999,
            &500,
            &100,
        ),
        Err(Ok(DikeError::InvalidInput))
    ));
    assert_eq!(registry.get_status(&1), MarketStatus::Live);
}

#[test]
fn dispute_escalates_to_council_and_registry_resolution() {
    let h = setup();
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let registry = DikeMarketRegistryClient::new(&h.env, &h.registry_id);
    let vault = CollateralVaultClient::new(&h.env, &h.vault_id);
    let council = CouncilOfDikeClient::new(&h.env, &h.council_id);
    let proposer_start = TokenClient::new(&h.env, &h.token).balance(&h.proposer);
    let disputer_start = TokenClient::new(&h.env, &h.token).balance(&h.disputer);
    let request_id = oracle.request_resolution(
        &1,
        &BytesN::from_array(&h.env, &[1; 32]),
        &String::from_str(&h.env, "ipfs://rules"),
        &999,
        &500,
        &100,
    );
    oracle.propose_outcome(
        &h.proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&h.env, "ipfs://yes"),
    );
    oracle.dispute_outcome(
        &h.disputer,
        &request_id,
        &Outcome::No,
        &String::from_str(&h.env, "ipfs://no"),
    );
    assert_eq!(registry.get_status(&1), MarketStatus::Disputed);
    assert_eq!(vault.accounting(&1).proposal_bonds, 500);
    assert_eq!(vault.accounting(&1).dispute_bonds, 500);

    oracle.escalate_to_council(&request_id);
    assert_eq!(registry.get_status(&1), MarketStatus::CouncilVoting);
    h.env.ledger().set_timestamp(1_301);
    assert_eq!(council.finalize_and_report_case(&1), Outcome::Invalid);
    assert_eq!(registry.get_status(&1), MarketStatus::Resolved);
    assert_eq!(registry.get_final_outcome(&1), Outcome::Invalid);
    assert_eq!(vault.accounting(&1).proposal_bonds, 0);
    assert_eq!(vault.accounting(&1).dispute_bonds, 0);
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.proposer),
        proposer_start
    );
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.disputer),
        disputer_start
    );
}

#[test]
fn council_win_splits_losing_bond_winner_council_treasury() {
    let h = setup();
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let vault = CollateralVaultClient::new(&h.env, &h.vault_id);
    let council = CouncilOfDikeClient::new(&h.env, &h.council_id);
    let gov = Address::generate(&h.env);
    let treasury = Address::generate(&h.env);
    let fee_manager_id = h.env.register(
        FeeManager,
        (&Address::generate(&h.env), &gov, &500i128, &100u32),
    );
    oracle.set_role(&symbol_short!("fees"), &fee_manager_id);
    oracle.set_role(&symbol_short!("treas"), &treasury);

    let proposer_start = TokenClient::new(&h.env, &h.token).balance(&h.proposer);
    let disputer_start = TokenClient::new(&h.env, &h.token).balance(&h.disputer);
    let request_id = oracle.request_resolution(
        &1,
        &BytesN::from_array(&h.env, &[1; 32]),
        &String::from_str(&h.env, "ipfs://rules"),
        &999,
        &500,
        &100,
    );
    oracle.propose_outcome(
        &h.proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&h.env, "ipfs://yes"),
    );
    oracle.dispute_outcome(
        &h.disputer,
        &request_id,
        &Outcome::No,
        &String::from_str(&h.env, "ipfs://no"),
    );
    oracle.escalate_to_council(&request_id);

    // Council rules in favor of the proposer's outcome: disputer's bond
    // (the losing bond) must split 60/30/10 winner/council/treasury
    // instead of the full amount going to the proposer.
    oracle.report_council_outcome(&request_id, &Outcome::Yes);

    assert_eq!(vault.accounting(&1).proposal_bonds, 0);
    assert_eq!(vault.accounting(&1).dispute_bonds, 0);
    // Proposer gets back their own 500 bond plus the 300 (60%) winner share.
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.proposer),
        proposer_start + 300
    );
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.disputer),
        disputer_start - 500
    );
    assert_eq!(TokenClient::new(&h.env, &h.token).balance(&treasury), 50);
    assert_eq!(
        TokenClient::new(&h.env, &h.token).balance(&h.council_id),
        150
    );

    let case_id = council.case_for_request(&request_id);
    let member = Address::generate(&h.env);
    council.set_role(&symbol_short!("gov"), &gov);
    council.set_member(&member, &true);
    let salt = BytesN::from_array(&h.env, &[3; 32]);
    let commitment = council.vote_commitment(&case_id, &member, &Outcome::Yes, &salt);
    council.commit_vote(&member, &case_id, &commitment);
    // Case was opened at t=1000 with commit/reveal windows of 100s each
    // (mirroring the request's dispute_window): commit_end=1100, reveal_end=1200.
    h.env.ledger().set_timestamp(1_101);
    council.reveal_vote(&member, &case_id, &Outcome::Yes, &salt);
    h.env.ledger().set_timestamp(1_201);
    // finalize_case here just tallies revealed votes into a status; it does
    // not re-run report_council_outcome since the request is already Finalized.
    council.finalize_case(&case_id);
    let (correct, payout) = council.claim_reward(&member, &case_id);
    assert!(correct);
    assert_eq!(payout, 150);
    assert_eq!(TokenClient::new(&h.env, &h.token).balance(&member), 150);
}

// --- Item H-2: mid-sequence bond-distribution revert proof ---

/// Forces a failure AFTER the first vault call in report_council_outcome succeeds
/// (release_bond for the proposer), so we can verify Soroban's transaction-level
/// atomicity reverts that first call along with everything else.
///
/// Setup: drain the disputer's bond directly before calling report_council_outcome
/// in the outcome==proposed_outcome branch.  The sequence inside the function is:
///   1. release_bond(proposer, false)            ← succeeds (proposal_bonds: 500→0 temporarily)
///   2. slash_bond(disputer, winner_amt, true, …) ← FAILS (disputer bond = 0)
///   → trap propagates → Soroban reverts entire tx → proposal_bonds restored to 500.
#[test]
fn bond_distribution_failure_mid_sequence_reverts_all() {
    let h = setup();
    let gov = Address::generate(&h.env);
    let treasury = Address::generate(&h.env);
    let fee_manager_id = h.env.register(
        FeeManager,
        (&Address::generate(&h.env), &gov, &500i128, &100u32),
    );
    let oracle = CODOracleClient::new(&h.env, &h.oracle_id);
    let vault = CollateralVaultClient::new(&h.env, &h.vault_id);
    oracle.set_role(&symbol_short!("fees"), &fee_manager_id);
    oracle.set_role(&symbol_short!("treas"), &treasury);

    // Full flow: request → propose → dispute → escalate
    let request_id = oracle.request_resolution(
        &1,
        &BytesN::from_array(&h.env, &[1; 32]),
        &String::from_str(&h.env, "ipfs://rules"),
        &999,
        &500,
        &100,
    );
    oracle.propose_outcome(
        &h.proposer,
        &request_id,
        &Outcome::Yes,
        &String::from_str(&h.env, "ipfs://yes"),
    );
    oracle.dispute_outcome(
        &h.disputer,
        &request_id,
        &Outcome::No,
        &String::from_str(&h.env, "ipfs://no"),
    );
    oracle.escalate_to_council(&request_id);

    assert_eq!(vault.accounting(&1).proposal_bonds, 500);
    assert_eq!(vault.accounting(&1).dispute_bonds, 500);
    assert_eq!(oracle.request(&request_id).status, OracleStatus::Escalated);

    // Drain the disputer's dispute bond directly (oracle role bypasses auth via
    // mock_all_auths).  This leaves dispute_bonds=0 and the disputer bond key
    // at zero, so the subsequent slash_bond call inside report_council_outcome
    // will fail with InsufficientBalance.
    vault.release_bond(&h.token, &h.disputer, &request_id, &500, &true);
    assert_eq!(vault.accounting(&1).dispute_bonds, 0);

    // outcome == proposed_outcome branch:
    //   step 1: release_bond(proposer) succeeds
    //   step 2: slash_bond(disputer, winner_amt) FAILS → trap → tx reverts step 1
    assert!(oracle
        .try_report_council_outcome(&request_id, &Outcome::Yes)
        .is_err());

    // Proposal bonds restored (step 1 was reverted).
    assert_eq!(vault.accounting(&1).proposal_bonds, 500);
    // Request still Escalated — the status write was reverted too.
    assert_eq!(oracle.request(&request_id).status, OracleStatus::Escalated);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let id = env.register(CODOracle, (&admin,));
    let client = CODOracleClient::new(&env, &id);
    // gov role not configured → pause returns Unauthorized
    assert!(matches!(
        client.try_pause(&true),
        Err(Ok(DikeError::Unauthorized))
    ));
    // council role not configured → report_council_outcome returns Unauthorized
    assert!(matches!(
        client.try_report_council_outcome(&1, &Outcome::Yes),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths — require_auth panics
    let admin = Address::generate(&env);
    let id = env.register(CODOracle, (&admin,));
    let client = CODOracleClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_role(&symbol_short!("gov"), &other).is_err());
}

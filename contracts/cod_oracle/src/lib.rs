#![no_std]
#![allow(clippy::too_many_arguments)]

use dike_types::{
    CouncilCase, DikeError, MarketData, MarketStatus, OpenCaseConfig, OracleStatus, Outcome,
    ResolutionRequest,
};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short, Address,
    BytesN, Env, String, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Request(u64),
    MarketRequest(u64),
    NextRequestId,
    Paused,
}

#[contractevent(topics = ["role"], data_format = "single-value")]
#[derive(Clone)]
pub struct RoleSet {
    #[topic]
    pub role: Symbol,
    pub module: Address,
}

#[contractevent(topics = ["admin"], data_format = "single-value")]
#[derive(Clone)]
pub struct AdminSet {
    pub admin: Address,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent(topics = ["res_req"], data_format = "single-value")]
#[derive(Clone)]
pub struct ResolutionRequested {
    #[topic]
    pub market_id: u64,
    pub request_id: u64,
}

#[contractevent(topics = ["propose"], data_format = "single-value")]
#[derive(Clone)]
pub struct OutcomeProposed {
    #[topic]
    pub request_id: u64,
    #[topic]
    pub proposer: Address,
    pub outcome: Outcome,
}

#[contractevent(topics = ["dispute"], data_format = "single-value")]
#[derive(Clone)]
pub struct OutcomeDisputed {
    #[topic]
    pub request_id: u64,
    #[topic]
    pub disputer: Address,
    pub outcome: Outcome,
}

#[contractevent(topics = ["final"], data_format = "single-value")]
#[derive(Clone)]
pub struct RequestFinalized {
    #[topic]
    pub request_id: u64,
    pub outcome: Outcome,
}

#[contractevent(topics = ["escal"])]
#[derive(Clone)]
pub struct RequestEscalated {
    #[topic]
    pub request_id: u64,
}

#[contractevent(topics = ["cod_fin"], data_format = "single-value")]
#[derive(Clone)]
pub struct CouncilOutcomeReported {
    #[topic]
    pub request_id: u64,
    pub outcome: Outcome,
}

#[contract]
pub struct CODOracle;

#[contractclient(name = "DikeRegistryClient")]
pub trait DikeRegistry {
    fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError>;
    fn close_trading(env: Env, market_id: u64) -> Result<(), DikeError>;
    fn mark_resolution_requested(
        env: Env,
        market_id: u64,
        request_id: u64,
    ) -> Result<(), DikeError>;
    fn mark_proposed(env: Env, market_id: u64) -> Result<(), DikeError>;
    fn mark_disputed(env: Env, market_id: u64) -> Result<(), DikeError>;
    fn mark_council_voting(env: Env, market_id: u64) -> Result<(), DikeError>;
    fn set_final_outcome(env: Env, market_id: u64, outcome: Outcome) -> Result<(), DikeError>;
}

#[contractclient(name = "DikeVaultClient")]
pub trait DikeVault {
    fn lock_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        market_id: u64,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError>;
    fn release_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        amount: i128,
        is_dispute: bool,
    ) -> Result<(), DikeError>;
    fn slash_bond(
        env: Env,
        token: Address,
        user: Address,
        request_id: u64,
        amount: i128,
        is_dispute: bool,
        recipient: Address,
    ) -> Result<(), DikeError>;
}

#[contractclient(name = "CouncilClient")]
pub trait Council {
    fn open_case(
        env: Env,
        request_id: u64,
        market_id: u64,
        proposer: Address,
        proposer_outcome: Outcome,
        proposer_evidence_uri: String,
        disputer: Address,
        disputer_outcome: Outcome,
        disputer_evidence_uri: String,
        config: OpenCaseConfig,
    ) -> Result<u64, DikeError>;
    fn case(env: Env, case_id: u64) -> Result<CouncilCase, DikeError>;
    fn case_for_request(env: Env, request_id: u64) -> Result<u64, DikeError>;
    fn record_case_reward(env: Env, case_id: u64, amount: i128) -> Result<(), DikeError>;
}

#[contractclient(name = "FeeManagerClient")]
pub trait FeeManager {
    fn losing_bond_split(env: Env, losing_bond: i128) -> Result<(i128, i128, i128), DikeError>;
}

fn bump(env: &Env) {
    env.storage().instance().extend_ttl(MIN_TTL, EXTEND_TTL);
}

fn require_admin(env: &Env) -> Result<(), DikeError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(DikeError::NotInitialized)?;
    admin.require_auth();
    Ok(())
}

fn require_role(env: &Env, role: Symbol) -> Result<(), DikeError> {
    let module: Address = env
        .storage()
        .instance()
        .get(&DataKey::Role(role))
        .ok_or(DikeError::Unauthorized)?;
    module.require_auth();
    Ok(())
}

fn read_request(env: &Env, request_id: u64) -> Result<ResolutionRequest, DikeError> {
    let key = DataKey::Request(request_id);
    if !env.storage().persistent().has(&key) {
        return Err(DikeError::RequestNotFound);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(DikeError::RequestNotFound)
}

fn write_request(env: &Env, request: &ResolutionRequest) {
    let key = DataKey::Request(request.id);
    env.storage().persistent().set(&key, request);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

fn ensure_not_paused(env: &Env) -> Result<(), DikeError> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        Err(DikeError::InvalidStatus)
    } else {
        Ok(())
    }
}

fn read_role(env: &Env, role: Symbol) -> Result<Address, DikeError> {
    env.storage()
        .instance()
        .get(&DataKey::Role(role))
        .ok_or(DikeError::Unauthorized)
}

fn load_market(env: &Env, market_id: u64) -> Result<MarketData, DikeError> {
    let registry = read_role(env, symbol_short!("registry"))?;
    Ok(DikeRegistryClient::new(env, &registry).get_market(&market_id))
}

fn validate_request_matches_market(
    market: &MarketData,
    question_hash: &BytesN<32>,
    rules_uri: &String,
    expiry: u64,
    bond_amount: i128,
    dispute_window: u64,
) -> Result<(), DikeError> {
    if market.question_hash != *question_hash
        || market.rules_uri != *rules_uri
        || market.expiry != expiry
        || market.bond_amount != bond_amount
        || market.dispute_window != dispute_window
    {
        return Err(DikeError::InvalidInput);
    }
    Ok(())
}

fn dispute_deadline(proposed_at: u64, dispute_window: u64) -> Result<u64, DikeError> {
    proposed_at
        .checked_add(dispute_window)
        .ok_or(DikeError::ArithmeticError)
}

#[contractimpl]
impl CODOracle {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextRequestId, &1u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        bump(&env);
    }

    pub fn set_admin(env: Env, admin: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        AdminSet { admin }.publish(&env);
        Ok(())
    }

    pub fn set_role(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Role(role.clone()), &module);
        RoleSet { role, module }.publish(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        Paused { paused }.publish(&env);
        Ok(())
    }

    pub fn request_resolution(
        env: Env,
        market_id: u64,
        question_hash: BytesN<32>,
        rules_uri: String,
        expiry: u64,
        bond_amount: i128,
        dispute_window: u64,
    ) -> Result<u64, DikeError> {
        // Intentional keeper path: anyone may request resolution once a market has
        // expired, because liveness matters more than proposer identity here.
        ensure_not_paused(&env)?;
        if rules_uri.is_empty() || bond_amount <= 0 || dispute_window == 0 {
            return Err(DikeError::InvalidInput);
        }
        let registry = read_role(&env, symbol_short!("registry"))?;
        let registry_client = DikeRegistryClient::new(&env, &registry);
        let market = registry_client.get_market(&market_id);
        validate_request_matches_market(
            &market,
            &question_hash,
            &rules_uri,
            expiry,
            bond_amount,
            dispute_window,
        )?;
        if env.ledger().timestamp() < market.expiry {
            return Err(DikeError::NotExpired);
        }
        if market.has_final_outcome
            || market.status == MarketStatus::Resolved
            || market.status == MarketStatus::Cancelled
        {
            return Err(DikeError::InvalidStatus);
        }
        if market.status == MarketStatus::Live {
            registry_client.close_trading(&market_id);
        } else if market.status != MarketStatus::TradingClosed {
            return Err(DikeError::InvalidStatus);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::MarketRequest(market_id))
        {
            return Err(DikeError::InvalidStatus);
        }
        let request_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextRequestId)
            .unwrap_or(1);
        let request = ResolutionRequest {
            id: request_id,
            market_id,
            question_hash,
            rules_uri,
            expiry,
            requested_at: env.ledger().timestamp(),
            bond_amount,
            dispute_window,
            has_proposal: false,
            proposer: env.current_contract_address(),
            proposed_outcome: Outcome::unset(),
            proposal_evidence_uri: String::from_str(&env, ""),
            proposed_at: 0,
            has_dispute: false,
            disputer: env.current_contract_address(),
            disputed_outcome: Outcome::unset(),
            dispute_evidence_uri: String::from_str(&env, ""),
            disputed_at: 0,
            status: OracleStatus::Requested,
            has_final_outcome: false,
            final_outcome: Outcome::unset(),
        };
        write_request(&env, &request);
        env.storage()
            .persistent()
            .set(&DataKey::MarketRequest(market_id), &request_id);
        env.storage().persistent().extend_ttl(
            &DataKey::MarketRequest(market_id),
            MIN_TTL,
            EXTEND_TTL,
        );
        let next_request_id = request_id
            .checked_add(1)
            .ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextRequestId, &next_request_id);
        registry_client.mark_resolution_requested(&market_id, &request_id);
        ResolutionRequested {
            market_id,
            request_id,
        }
        .publish(&env);
        Ok(request_id)
    }

    pub fn propose_outcome(
        env: Env,
        proposer: Address,
        request_id: u64,
        outcome: Outcome,
        evidence_uri: String,
    ) -> Result<(), DikeError> {
        proposer.require_auth();
        if evidence_uri.is_empty() {
            return Err(DikeError::EvidenceRequired);
        }
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Requested {
            return Err(DikeError::InvalidStatus);
        }
        let _ = dispute_deadline(env.ledger().timestamp(), request.dispute_window)?;
        let market = load_market(&env, request.market_id)?;
        let vault = read_role(&env, symbol_short!("vault"))?;
        DikeVaultClient::new(&env, &vault).lock_bond(
            &market.collateral,
            &proposer,
            &request_id,
            &request.market_id,
            &request.bond_amount,
            &false,
        );
        request.has_proposal = true;
        request.proposer = proposer.clone();
        request.proposed_outcome = outcome;
        request.proposal_evidence_uri = evidence_uri;
        request.proposed_at = env.ledger().timestamp();
        request.status = OracleStatus::Proposed;
        write_request(&env, &request);
        let registry = read_role(&env, symbol_short!("registry"))?;
        DikeRegistryClient::new(&env, &registry).mark_proposed(&request.market_id);
        OutcomeProposed {
            request_id,
            proposer,
            outcome,
        }
        .publish(&env);
        Ok(())
    }

    pub fn dispute_outcome(
        env: Env,
        disputer: Address,
        request_id: u64,
        counter_outcome: Outcome,
        evidence_uri: String,
    ) -> Result<(), DikeError> {
        disputer.require_auth();
        if evidence_uri.is_empty() {
            return Err(DikeError::EvidenceRequired);
        }
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Proposed {
            return Err(DikeError::InvalidStatus);
        }
        if !request.has_proposal {
            return Err(DikeError::InvalidStatus);
        }
        let proposed_at = request.proposed_at;
        if env.ledger().timestamp() > dispute_deadline(proposed_at, request.dispute_window)? {
            return Err(DikeError::DisputeWindowClosed);
        }
        if request.proposed_outcome == counter_outcome {
            return Err(DikeError::InvalidInput);
        }
        let market = load_market(&env, request.market_id)?;
        let vault = read_role(&env, symbol_short!("vault"))?;
        DikeVaultClient::new(&env, &vault).lock_bond(
            &market.collateral,
            &disputer,
            &request_id,
            &request.market_id,
            &request.bond_amount,
            &true,
        );
        request.has_dispute = true;
        request.disputer = disputer.clone();
        request.disputed_outcome = counter_outcome;
        request.dispute_evidence_uri = evidence_uri;
        request.disputed_at = env.ledger().timestamp();
        request.status = OracleStatus::Disputed;
        write_request(&env, &request);
        let registry = read_role(&env, symbol_short!("registry"))?;
        DikeRegistryClient::new(&env, &registry).mark_disputed(&request.market_id);
        OutcomeDisputed {
            request_id,
            disputer,
            outcome: counter_outcome,
        }
        .publish(&env);
        Ok(())
    }

    pub fn finalize_undisputed(env: Env, request_id: u64) -> Result<Outcome, DikeError> {
        // Intentional keeper path: once the dispute window has elapsed, any caller
        // may finalize the undisputed request to avoid stalled markets.
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Proposed {
            return Err(DikeError::InvalidStatus);
        }
        if !request.has_proposal {
            return Err(DikeError::InvalidStatus);
        }
        let proposed_at = request.proposed_at;
        if env.ledger().timestamp() <= dispute_deadline(proposed_at, request.dispute_window)? {
            return Err(DikeError::DisputeWindowOpen);
        }
        let outcome = request.proposed_outcome;
        let market = load_market(&env, request.market_id)?;
        let registry = read_role(&env, symbol_short!("registry"))?;
        DikeRegistryClient::new(&env, &registry).set_final_outcome(&request.market_id, &outcome);
        let vault = read_role(&env, symbol_short!("vault"))?;
        DikeVaultClient::new(&env, &vault).release_bond(
            &market.collateral,
            &request.proposer,
            &request_id,
            &request.bond_amount,
            &false,
        );
        request.has_final_outcome = true;
        request.final_outcome = outcome;
        request.status = OracleStatus::Finalized;
        write_request(&env, &request);
        RequestFinalized {
            request_id,
            outcome,
        }
        .publish(&env);
        Ok(outcome)
    }

    pub fn escalate_to_council(env: Env, request_id: u64) -> Result<(), DikeError> {
        // Intentional keeper path: any caller may push a disputed request into
        // council voting so the protocol cannot be griefed into a stuck state.
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Disputed {
            return Err(DikeError::InvalidStatus);
        }
        let market = load_market(&env, request.market_id)?;
        let council = read_role(&env, symbol_short!("council"))?;
        CouncilClient::new(&env, &council).open_case(
            &request.id,
            &request.market_id,
            &request.proposer,
            &request.proposed_outcome,
            &request.proposal_evidence_uri,
            &request.disputer,
            &request.disputed_outcome,
            &request.dispute_evidence_uri,
            &OpenCaseConfig {
                proposal_bond: request.bond_amount,
                dispute_bond: request.bond_amount,
                commit_duration: request.dispute_window,
                reveal_duration: request.dispute_window,
                token: market.collateral,
            },
        );
        let registry = read_role(&env, symbol_short!("registry"))?;
        DikeRegistryClient::new(&env, &registry).mark_council_voting(&request.market_id);
        request.status = OracleStatus::Escalated;
        write_request(&env, &request);
        RequestEscalated { request_id }.publish(&env);
        Ok(())
    }

    /// Atomicity guarantee for the sequential bond distribution below:
    ///
    /// `vault_client.release_bond` and `vault_client.slash_bond` are called via
    /// the generated non-Try client.  Any `Err` or trap returned by a nested
    /// call propagates as a trap/panic, which causes Soroban to revert the
    /// *entire* top-level transaction write-set — including storage mutations and
    /// token transfers from all prior steps in this same invocation.  Therefore
    /// bonds can never be partially distributed: either every slash/release in
    /// this function commits atomically, or none of them do.
    pub fn report_council_outcome(
        env: Env,
        request_id: u64,
        outcome: Outcome,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("council"))?;
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Escalated {
            return Err(DikeError::InvalidStatus);
        }
        if request.has_final_outcome {
            return Err(DikeError::AlreadyResolved);
        }
        let market = load_market(&env, request.market_id)?;
        let vault = read_role(&env, symbol_short!("vault"))?;
        let vault_client = DikeVaultClient::new(&env, &vault);
        if outcome == request.proposed_outcome {
            vault_client.release_bond(
                &market.collateral,
                &request.proposer,
                &request_id,
                &request.bond_amount,
                &false,
            );
            let fee_manager = read_role(&env, symbol_short!("fees"))?;
            let treasury = read_role(&env, symbol_short!("treas"))?;
            let council = read_role(&env, symbol_short!("council"))?;
            let (winner_amt, council_amt, treasury_amt) =
                FeeManagerClient::new(&env, &fee_manager).losing_bond_split(&request.bond_amount);
            vault_client.slash_bond(
                &market.collateral,
                &request.disputer,
                &request_id,
                &winner_amt,
                &true,
                &request.proposer,
            );
            if council_amt > 0 {
                vault_client.slash_bond(
                    &market.collateral,
                    &request.disputer,
                    &request_id,
                    &council_amt,
                    &true,
                    &council,
                );
                let case_id = CouncilClient::new(&env, &council).case_for_request(&request_id);
                CouncilClient::new(&env, &council).record_case_reward(&case_id, &council_amt);
            }
            if treasury_amt > 0 {
                vault_client.slash_bond(
                    &market.collateral,
                    &request.disputer,
                    &request_id,
                    &treasury_amt,
                    &true,
                    &treasury,
                );
            }
        } else if outcome == request.disputed_outcome {
            vault_client.release_bond(
                &market.collateral,
                &request.disputer,
                &request_id,
                &request.bond_amount,
                &true,
            );
            let fee_manager = read_role(&env, symbol_short!("fees"))?;
            let treasury = read_role(&env, symbol_short!("treas"))?;
            let council = read_role(&env, symbol_short!("council"))?;
            let (winner_amt, council_amt, treasury_amt) =
                FeeManagerClient::new(&env, &fee_manager).losing_bond_split(&request.bond_amount);
            vault_client.slash_bond(
                &market.collateral,
                &request.proposer,
                &request_id,
                &winner_amt,
                &false,
                &request.disputer,
            );
            if council_amt > 0 {
                vault_client.slash_bond(
                    &market.collateral,
                    &request.proposer,
                    &request_id,
                    &council_amt,
                    &false,
                    &council,
                );
                let case_id = CouncilClient::new(&env, &council).case_for_request(&request_id);
                CouncilClient::new(&env, &council).record_case_reward(&case_id, &council_amt);
            }
            if treasury_amt > 0 {
                vault_client.slash_bond(
                    &market.collateral,
                    &request.proposer,
                    &request_id,
                    &treasury_amt,
                    &false,
                    &treasury,
                );
            }
        } else {
            vault_client.release_bond(
                &market.collateral,
                &request.proposer,
                &request_id,
                &request.bond_amount,
                &false,
            );
            vault_client.release_bond(
                &market.collateral,
                &request.disputer,
                &request_id,
                &request.bond_amount,
                &true,
            );
        }
        let registry = read_role(&env, symbol_short!("registry"))?;
        DikeRegistryClient::new(&env, &registry).set_final_outcome(&request.market_id, &outcome);
        request.has_final_outcome = true;
        request.final_outcome = outcome;
        request.status = OracleStatus::Finalized;
        write_request(&env, &request);
        CouncilOutcomeReported {
            request_id,
            outcome,
        }
        .publish(&env);
        Ok(())
    }

    pub fn request(env: Env, request_id: u64) -> Result<ResolutionRequest, DikeError> {
        read_request(&env, request_id)
    }

    pub fn market_request(env: Env, market_id: u64) -> Result<u64, DikeError> {
        let key = DataKey::MarketRequest(market_id);
        if !env.storage().persistent().has(&key) {
            return Err(DikeError::RequestNotFound);
        }
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(DikeError::RequestNotFound)
    }
}

mod test;

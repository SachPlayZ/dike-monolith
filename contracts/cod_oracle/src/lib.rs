#![no_std]

use dike_types::{DikeError, MarketId, OracleStatus, Outcome, RequestId, ResolutionRequest};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, String, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Request(RequestId),
    MarketRequest(MarketId),
    NextRequestId,
    Paused,
}

#[contract]
pub struct CODOracle;

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

fn read_request(env: &Env, request_id: RequestId) -> Result<ResolutionRequest, DikeError> {
    let key = DataKey::Request(request_id);
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

    pub fn set_role(env: Env, role: Symbol, module: Address) -> Result<(), DikeError> {
        require_admin(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::Role(role.clone()), &module);
        env.events().publish((symbol_short!("role"), role), module);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish((symbol_short!("pause"),), paused);
        Ok(())
    }

    pub fn request_resolution(
        env: Env,
        market_id: MarketId,
        question_hash: BytesN<32>,
        rules_uri: String,
        expiry: u64,
        bond_amount: i128,
        dispute_window: u64,
    ) -> Result<RequestId, DikeError> {
        ensure_not_paused(&env)?;
        if env.ledger().timestamp() < expiry {
            return Err(DikeError::NotExpired);
        }
        if rules_uri.len() == 0 || bond_amount <= 0 || dispute_window == 0 {
            return Err(DikeError::InvalidInput);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::MarketRequest(market_id))
        {
            return Err(DikeError::InvalidStatus);
        }
        let request_id: RequestId = env
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
            proposer: None,
            proposed_outcome: None,
            proposal_evidence_uri: None,
            proposed_at: None,
            disputer: None,
            disputed_outcome: None,
            dispute_evidence_uri: None,
            disputed_at: None,
            status: OracleStatus::Requested,
            final_outcome: None,
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
        env.storage()
            .instance()
            .set(&DataKey::NextRequestId, &(request_id + 1));
        env.events()
            .publish((symbol_short!("res_req"), market_id), request_id);
        Ok(request_id)
    }

    pub fn propose_outcome(
        env: Env,
        proposer: Address,
        request_id: RequestId,
        outcome: Outcome,
        evidence_uri: String,
    ) -> Result<(), DikeError> {
        ensure_not_paused(&env)?;
        proposer.require_auth();
        if evidence_uri.len() == 0 {
            return Err(DikeError::EvidenceRequired);
        }
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Requested {
            return Err(DikeError::InvalidStatus);
        }
        request.proposer = Some(proposer.clone());
        request.proposed_outcome = Some(outcome);
        request.proposal_evidence_uri = Some(evidence_uri);
        request.proposed_at = Some(env.ledger().timestamp());
        request.status = OracleStatus::Proposed;
        write_request(&env, &request);
        env.events()
            .publish((symbol_short!("propose"), request_id, proposer), outcome);
        Ok(())
    }

    pub fn dispute_outcome(
        env: Env,
        disputer: Address,
        request_id: RequestId,
        counter_outcome: Outcome,
        evidence_uri: String,
    ) -> Result<(), DikeError> {
        ensure_not_paused(&env)?;
        disputer.require_auth();
        if evidence_uri.len() == 0 {
            return Err(DikeError::EvidenceRequired);
        }
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Proposed {
            return Err(DikeError::InvalidStatus);
        }
        let proposed_at = request.proposed_at.ok_or(DikeError::InvalidStatus)?;
        if env.ledger().timestamp() > proposed_at + request.dispute_window {
            return Err(DikeError::DisputeWindowClosed);
        }
        if request.proposed_outcome == Some(counter_outcome) {
            return Err(DikeError::InvalidInput);
        }
        request.disputer = Some(disputer.clone());
        request.disputed_outcome = Some(counter_outcome);
        request.dispute_evidence_uri = Some(evidence_uri);
        request.disputed_at = Some(env.ledger().timestamp());
        request.status = OracleStatus::Disputed;
        write_request(&env, &request);
        env.events().publish(
            (symbol_short!("dispute"), request_id, disputer),
            counter_outcome,
        );
        Ok(())
    }

    pub fn finalize_undisputed(env: Env, request_id: RequestId) -> Result<Outcome, DikeError> {
        ensure_not_paused(&env)?;
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Proposed {
            return Err(DikeError::InvalidStatus);
        }
        let proposed_at = request.proposed_at.ok_or(DikeError::InvalidStatus)?;
        if env.ledger().timestamp() <= proposed_at + request.dispute_window {
            return Err(DikeError::DisputeWindowOpen);
        }
        let outcome = request.proposed_outcome.ok_or(DikeError::InvalidStatus)?;
        request.final_outcome = Some(outcome);
        request.status = OracleStatus::Finalized;
        write_request(&env, &request);
        env.events()
            .publish((symbol_short!("final"), request_id), outcome);
        Ok(outcome)
    }

    pub fn escalate_to_council(env: Env, request_id: RequestId) -> Result<(), DikeError> {
        ensure_not_paused(&env)?;
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Disputed {
            return Err(DikeError::InvalidStatus);
        }
        request.status = OracleStatus::Escalated;
        write_request(&env, &request);
        env.events()
            .publish((symbol_short!("escal"), request_id), ());
        Ok(())
    }

    pub fn report_council_outcome(
        env: Env,
        request_id: RequestId,
        outcome: Outcome,
    ) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("council"))?;
        let mut request = read_request(&env, request_id)?;
        if request.status != OracleStatus::Escalated {
            return Err(DikeError::InvalidStatus);
        }
        if request.final_outcome.is_some() {
            return Err(DikeError::AlreadyResolved);
        }
        request.final_outcome = Some(outcome);
        request.status = OracleStatus::Finalized;
        write_request(&env, &request);
        env.events()
            .publish((symbol_short!("cod_fin"), request_id), outcome);
        Ok(())
    }

    pub fn request(env: Env, request_id: RequestId) -> Result<ResolutionRequest, DikeError> {
        read_request(&env, request_id)
    }

    pub fn market_request(env: Env, market_id: MarketId) -> Result<RequestId, DikeError> {
        let key = DataKey::MarketRequest(market_id);
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

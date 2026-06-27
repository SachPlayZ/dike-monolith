#![no_std]

use dike_types::{
    CaseId, CouncilCase, CouncilCaseStatus, DikeError, MarketId, OpenCaseConfig, Outcome, RequestId,
};
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
    Member(Address),
    Case(CaseId),
    RequestCase(RequestId),
    NextCaseId,
    Commit(CaseId, Address),
    Reveal(CaseId, Address),
    Claimed(CaseId, Address),
    Paused,
}

#[contract]
pub struct CouncilOfDike;

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

fn require_member(env: &Env, voter: &Address) -> Result<(), DikeError> {
    let is_member: bool = env
        .storage()
        .instance()
        .get(&DataKey::Member(voter.clone()))
        .unwrap_or(false);
    if !is_member {
        return Err(DikeError::Unauthorized);
    }
    voter.require_auth();
    Ok(())
}

fn read_case(env: &Env, case_id: CaseId) -> Result<CouncilCase, DikeError> {
    let key = DataKey::Case(case_id);
    if !env.storage().persistent().has(&key) {
        return Err(DikeError::CaseNotFound);
    }
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(DikeError::CaseNotFound)
}

fn write_case(env: &Env, case_data: &CouncilCase) {
    let key = DataKey::Case(case_data.id);
    env.storage().persistent().set(&key, case_data);
    env.storage()
        .persistent()
        .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
}

#[contractimpl]
impl CouncilOfDike {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextCaseId, &1u64);
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

    pub fn set_member(env: Env, member: Address, approved: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage()
            .instance()
            .set(&DataKey::Member(member.clone()), &approved);
        env.events()
            .publish((symbol_short!("member"), member), approved);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish((symbol_short!("pause"),), paused);
        Ok(())
    }

    pub fn open_case(
        env: Env,
        request_id: RequestId,
        market_id: MarketId,
        proposer: Address,
        proposer_outcome: Outcome,
        proposer_evidence_uri: String,
        disputer: Address,
        disputer_outcome: Outcome,
        disputer_evidence_uri: String,
        config: OpenCaseConfig,
    ) -> Result<CaseId, DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if proposer_evidence_uri.len() == 0 || disputer_evidence_uri.len() == 0 {
            return Err(DikeError::EvidenceRequired);
        }
        if proposer_outcome == disputer_outcome
            || config.proposal_bond <= 0
            || config.dispute_bond <= 0
        {
            return Err(DikeError::InvalidInput);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::RequestCase(request_id))
        {
            return Err(DikeError::InvalidStatus);
        }
        let case_id: CaseId = env
            .storage()
            .instance()
            .get(&DataKey::NextCaseId)
            .unwrap_or(1);
        let now = env.ledger().timestamp();
        let case_data = CouncilCase {
            id: case_id,
            request_id,
            market_id,
            proposer,
            proposer_outcome,
            proposer_evidence_uri,
            disputer,
            disputer_outcome,
            disputer_evidence_uri,
            proposal_bond: config.proposal_bond,
            dispute_bond: config.dispute_bond,
            voting_start: now,
            commit_end: now + config.commit_duration,
            reveal_end: now + config.commit_duration + config.reveal_duration,
            status: CouncilCaseStatus::CommitPhase,
            has_final_outcome: false,
            final_outcome: Outcome::unset(),
            yes_votes: 0,
            no_votes: 0,
            invalid_votes: 0,
            total_valid_votes: 0,
        };
        write_case(&env, &case_data);
        env.storage()
            .persistent()
            .set(&DataKey::RequestCase(request_id), &case_id);
        env.storage().persistent().extend_ttl(
            &DataKey::RequestCase(request_id),
            MIN_TTL,
            EXTEND_TTL,
        );
        env.storage()
            .instance()
            .set(&DataKey::NextCaseId, &(case_id + 1));
        env.events()
            .publish((symbol_short!("case"), request_id), case_id);
        Ok(case_id)
    }

    pub fn commit_vote(
        env: Env,
        voter: Address,
        case_id: CaseId,
        commitment: BytesN<32>,
    ) -> Result<(), DikeError> {
        require_member(&env, &voter)?;
        let case_data = read_case(&env, case_id)?;
        if case_data.status != CouncilCaseStatus::CommitPhase
            || env.ledger().timestamp() > case_data.commit_end
        {
            return Err(DikeError::InvalidStatus);
        }
        let key = DataKey::Commit(case_id, voter.clone());
        if env.storage().persistent().has(&key) {
            return Err(DikeError::VoteAlreadyCommitted);
        }
        env.storage().persistent().set(&key, &commitment);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.events()
            .publish((symbol_short!("commit"), case_id, voter), commitment);
        Ok(())
    }

    pub fn reveal_vote(
        env: Env,
        voter: Address,
        case_id: CaseId,
        outcome: Outcome,
        commitment: BytesN<32>,
    ) -> Result<(), DikeError> {
        require_member(&env, &voter)?;
        let mut case_data = read_case(&env, case_id)?;
        let now = env.ledger().timestamp();
        if now <= case_data.commit_end || now > case_data.reveal_end {
            return Err(DikeError::InvalidStatus);
        }
        let commit_key = DataKey::Commit(case_id, voter.clone());
        let stored: BytesN<32> = env
            .storage()
            .persistent()
            .get(&commit_key)
            .ok_or(DikeError::VoteNotCommitted)?;
        if stored != commitment {
            return Err(DikeError::InvalidReveal);
        }
        let reveal_key = DataKey::Reveal(case_id, voter.clone());
        if env.storage().persistent().has(&reveal_key) {
            return Err(DikeError::InvalidReveal);
        }
        match outcome {
            Outcome::Yes => case_data.yes_votes += 1,
            Outcome::No => case_data.no_votes += 1,
            Outcome::Invalid => case_data.invalid_votes += 1,
        }
        case_data.total_valid_votes += 1;
        env.storage().persistent().set(&reveal_key, &outcome);
        env.storage()
            .persistent()
            .extend_ttl(&reveal_key, MIN_TTL, EXTEND_TTL);
        write_case(&env, &case_data);
        env.events()
            .publish((symbol_short!("reveal"), case_id, voter), outcome);
        Ok(())
    }

    pub fn finalize_case(env: Env, case_id: CaseId) -> Result<Outcome, DikeError> {
        let mut case_data = read_case(&env, case_id)?;
        if case_data.has_final_outcome {
            return Err(DikeError::AlreadyResolved);
        }
        if env.ledger().timestamp() <= case_data.reveal_end {
            return Err(DikeError::TooEarly);
        }
        let outcome = if case_data.yes_votes > case_data.no_votes
            && case_data.yes_votes > case_data.invalid_votes
        {
            Outcome::Yes
        } else if case_data.no_votes > case_data.yes_votes
            && case_data.no_votes > case_data.invalid_votes
        {
            Outcome::No
        } else {
            Outcome::Invalid
        };
        case_data.has_final_outcome = true;
        case_data.final_outcome = outcome;
        case_data.status = CouncilCaseStatus::Finalized;
        write_case(&env, &case_data);
        env.events()
            .publish((symbol_short!("casefin"), case_id), outcome);
        Ok(outcome)
    }

    pub fn claim_reward(env: Env, voter: Address, case_id: CaseId) -> Result<bool, DikeError> {
        voter.require_auth();
        let case_data = read_case(&env, case_id)?;
        if !case_data.has_final_outcome {
            return Err(DikeError::InvalidStatus);
        }
        let final_outcome = case_data.final_outcome;
        let reveal_key = DataKey::Reveal(case_id, voter.clone());
        let revealed: Outcome = env
            .storage()
            .persistent()
            .get(&reveal_key)
            .ok_or(DikeError::VoteNotCommitted)?;
        let claimed_key = DataKey::Claimed(case_id, voter.clone());
        if env.storage().persistent().has(&claimed_key) {
            return Err(DikeError::ActionConsumed);
        }
        let correct = revealed == final_outcome;
        env.storage().persistent().set(&claimed_key, &correct);
        env.storage()
            .persistent()
            .extend_ttl(&claimed_key, MIN_TTL, EXTEND_TTL);
        env.events()
            .publish((symbol_short!("reward"), case_id, voter), correct);
        Ok(correct)
    }

    pub fn case(env: Env, case_id: CaseId) -> Result<CouncilCase, DikeError> {
        read_case(&env, case_id)
    }

    pub fn case_for_request(env: Env, request_id: RequestId) -> Result<CaseId, DikeError> {
        let key = DataKey::RequestCase(request_id);
        if !env.storage().persistent().has(&key) {
            return Err(DikeError::CaseNotFound);
        }
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(DikeError::CaseNotFound)
    }
}

mod test;

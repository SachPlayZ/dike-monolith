#![no_std]
#![allow(clippy::too_many_arguments)]

use dike_math::checked_add;
use dike_types::{CouncilCase, CouncilCaseStatus, DikeError, OpenCaseConfig, Outcome};
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short,
    token::Client as TokenClient, xdr::ToXdr, Address, BytesN, Env, String, Symbol,
};

const MIN_TTL: u32 = 17_280;
const EXTEND_TTL: u32 = 518_400;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol),
    Member(Address),
    Case(u64),
    RequestCase(u64),
    NextCaseId,
    Commit(u64, Address),
    Reveal(u64, Address),
    Claimed(u64, Address),
    CaseToken(u64),
    CaseRewardPool(u64),
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

#[contractevent(topics = ["member"], data_format = "single-value")]
#[derive(Clone)]
pub struct MemberSet {
    #[topic]
    pub member: Address,
    pub approved: bool,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent(topics = ["case"], data_format = "single-value")]
#[derive(Clone)]
pub struct CaseOpened {
    #[topic]
    pub request_id: u64,
    pub case_id: u64,
}

#[contractevent(topics = ["commit"], data_format = "single-value")]
#[derive(Clone)]
pub struct VoteCommitted {
    #[topic]
    pub case_id: u64,
    #[topic]
    pub voter: Address,
    pub commitment: BytesN<32>,
}

#[contractevent(topics = ["reveal"], data_format = "single-value")]
#[derive(Clone)]
pub struct VoteRevealed {
    #[topic]
    pub case_id: u64,
    #[topic]
    pub voter: Address,
    pub outcome: Outcome,
}

#[contractevent(topics = ["casefin"], data_format = "single-value")]
#[derive(Clone)]
pub struct CaseFinalized {
    #[topic]
    pub case_id: u64,
    pub outcome: Outcome,
}

#[contractevent(topics = ["reward"], data_format = "vec")]
#[derive(Clone)]
pub struct RewardClaimed {
    #[topic]
    pub case_id: u64,
    #[topic]
    pub voter: Address,
    pub correct: bool,
    pub payout: i128,
}

#[contract]
pub struct CouncilOfDike;

#[contractclient(name = "CODOracleClient")]
pub trait CODOracle {
    fn report_council_outcome(env: Env, request_id: u64, outcome: Outcome)
        -> Result<(), DikeError>;
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

fn ensure_not_paused(env: &Env) -> Result<(), DikeError> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        return Err(DikeError::InvalidStatus);
    }
    Ok(())
}

fn read_case(env: &Env, case_id: u64) -> Result<CouncilCase, DikeError> {
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

fn vote_commitment_hash(
    env: &Env,
    case_id: u64,
    voter: Address,
    outcome: Outcome,
    salt: BytesN<32>,
) -> BytesN<32> {
    env.crypto()
        .sha256(&(case_id, voter, outcome, salt).to_xdr(env))
        .to_bytes()
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

    pub fn set_member(env: Env, member: Address, approved: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage()
            .instance()
            .set(&DataKey::Member(member.clone()), &approved);
        MemberSet { member, approved }.publish(&env);
        Ok(())
    }

    pub fn pause(env: Env, paused: bool) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("gov"))?;
        env.storage().instance().set(&DataKey::Paused, &paused);
        Paused { paused }.publish(&env);
        Ok(())
    }

    pub fn record_case_reward(env: Env, case_id: u64, amount: i128) -> Result<(), DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if amount <= 0 {
            return Err(DikeError::InvalidAmount);
        }
        let key = DataKey::CaseRewardPool(case_id);
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let next = checked_add(current, amount)?;
        env.storage().persistent().set(&key, &next);
        env.storage()
            .persistent()
            .extend_ttl(&key, MIN_TTL, EXTEND_TTL);
        Ok(())
    }

    pub fn open_case(
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
    ) -> Result<u64, DikeError> {
        require_role(&env, symbol_short!("oracle"))?;
        if proposer_evidence_uri.is_empty() || disputer_evidence_uri.is_empty() {
            return Err(DikeError::EvidenceRequired);
        }
        if proposer_outcome == disputer_outcome
            || config.proposal_bond <= 0
            || config.dispute_bond <= 0
            || config.commit_duration == 0
            || config.reveal_duration == 0
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
        let case_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextCaseId)
            .unwrap_or(1);
        let now = env.ledger().timestamp();
        let commit_end = now
            .checked_add(config.commit_duration)
            .ok_or(DikeError::ArithmeticError)?;
        let reveal_end = commit_end
            .checked_add(config.reveal_duration)
            .ok_or(DikeError::ArithmeticError)?;
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
            commit_end,
            reveal_end,
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
        let token_key = DataKey::CaseToken(case_id);
        env.storage().persistent().set(&token_key, &config.token);
        env.storage()
            .persistent()
            .extend_ttl(&token_key, MIN_TTL, EXTEND_TTL);
        let next_case_id = case_id.checked_add(1).ok_or(DikeError::ArithmeticError)?;
        env.storage()
            .instance()
            .set(&DataKey::NextCaseId, &next_case_id);
        CaseOpened {
            request_id,
            case_id,
        }
        .publish(&env);
        Ok(case_id)
    }

    pub fn commit_vote(
        env: Env,
        voter: Address,
        case_id: u64,
        commitment: BytesN<32>,
    ) -> Result<(), DikeError> {
        ensure_not_paused(&env)?;
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
        VoteCommitted {
            case_id,
            voter,
            commitment,
        }
        .publish(&env);
        Ok(())
    }

    pub fn reveal_vote(
        env: Env,
        voter: Address,
        case_id: u64,
        outcome: Outcome,
        salt: BytesN<32>,
    ) -> Result<(), DikeError> {
        ensure_not_paused(&env)?;
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
        if stored != vote_commitment_hash(&env, case_id, voter.clone(), outcome, salt) {
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
        VoteRevealed {
            case_id,
            voter,
            outcome,
        }
        .publish(&env);
        Ok(())
    }

    pub fn finalize_case(env: Env, case_id: u64) -> Result<Outcome, DikeError> {
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
        CaseFinalized { case_id, outcome }.publish(&env);
        Ok(outcome)
    }

    pub fn finalize_and_report_case(env: Env, case_id: u64) -> Result<Outcome, DikeError> {
        let outcome = Self::finalize_case(env.clone(), case_id)?;
        let case_data = read_case(&env, case_id)?;
        let oracle: Address = env
            .storage()
            .instance()
            .get(&DataKey::Role(symbol_short!("oracle")))
            .ok_or(DikeError::Unauthorized)?;
        CODOracleClient::new(&env, &oracle).report_council_outcome(&case_data.request_id, &outcome);
        Ok(outcome)
    }

    pub fn claim_reward(env: Env, voter: Address, case_id: u64) -> Result<(bool, i128), DikeError> {
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

        let mut payout: i128 = 0;
        if correct {
            let reward_pool: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::CaseRewardPool(case_id))
                .unwrap_or(0);
            let correct_votes = match final_outcome {
                Outcome::Yes => case_data.yes_votes,
                Outcome::No => case_data.no_votes,
                Outcome::Invalid => case_data.invalid_votes,
            } as i128;
            if reward_pool > 0 && correct_votes > 0 {
                payout = reward_pool / correct_votes;
                if payout > 0 {
                    let token: Address = env
                        .storage()
                        .persistent()
                        .get(&DataKey::CaseToken(case_id))
                        .ok_or(DikeError::NotInitialized)?;
                    TokenClient::new(&env, &token).transfer(
                        &env.current_contract_address(),
                        &voter,
                        &payout,
                    );
                }
            }
        }

        RewardClaimed {
            case_id,
            voter,
            correct,
            payout,
        }
        .publish(&env);
        Ok((correct, payout))
    }

    pub fn case(env: Env, case_id: u64) -> Result<CouncilCase, DikeError> {
        read_case(&env, case_id)
    }

    pub fn case_reward_pool(env: Env, case_id: u64) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::CaseRewardPool(case_id))
            .unwrap_or(0)
    }

    pub fn case_for_request(env: Env, request_id: u64) -> Result<u64, DikeError> {
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

    pub fn vote_commitment(
        env: Env,
        case_id: u64,
        voter: Address,
        outcome: Outcome,
        salt: BytesN<32>,
    ) -> BytesN<32> {
        vote_commitment_hash(&env, case_id, voter, outcome, salt)
    }
}

mod test;

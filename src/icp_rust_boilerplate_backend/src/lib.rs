use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, hash::{Hash, Hasher}};
use ic_cdk::api::time;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PrincipalWrapper(Principal);

impl Hash for PrincipalWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Storable for PrincipalWrapper {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(self.0.as_slice().to_vec())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(Principal::from_slice(&bytes))
    }
}

impl BoundedStorable for PrincipalWrapper {
    const MAX_SIZE: u32 = 29;
    const IS_FIXED_SIZE: bool = false;
}

impl Default for PrincipalWrapper {
    fn default() -> Self {
        Self(Principal::anonymous())
    }
}

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
enum ProposalStatus {
    Pending,
    Active,
    Ended,
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
struct Proposal {
    id: u64,
    title: String,
    description: String,
    creator: PrincipalWrapper,
    created_at: u64,
    start_date: u64,
    end_date: u64,
    votes_for: u64,
    votes_against: u64,
    status: ProposalStatus,
}

impl Proposal {
    fn update_status(&mut self) {
        let current_time = time() / 1_000_000_000;
        self.status = if current_time < self.start_date {
            ProposalStatus::Pending
        } else if current_time >= self.start_date && current_time <= self.end_date {
            ProposalStatus::Active
        } else {
            ProposalStatus::Ended
        };
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone)]
struct Vote {
    user: PrincipalWrapper,
    proposal_id: u64,
    vote_power: u64,
    is_for: bool,
}

impl Storable for Proposal {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Proposal {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Vote {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Vote {
    const MAX_SIZE: u32 = 64;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<Cell<u64, VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        Cell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 1)
            .expect("failed to initialize ID counter")
    );

    static PROPOSALS: RefCell<StableBTreeMap<u64, Proposal, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))))
    );

    static VOTES: RefCell<StableBTreeMap<(PrincipalWrapper, u64), Vote, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))))
    );

    static USER_TOKENS: RefCell<HashMap<PrincipalWrapper, u64>> = RefCell::new(HashMap::new());
}

#[derive(CandidType, Deserialize)]
struct ProposalPayload {
    title: String,
    description: String,
    start_date: String,  // Format: DD-MM-YYYY
    end_date: String,    // Format: DD-MM-YYYY
}

#[derive(CandidType, Serialize, Deserialize)]
struct VotePayload {
    proposal_id: u64,
    is_for: bool,
    tokens: u64,
}

#[derive(CandidType, Serialize, Deserialize)]
enum Error {
    NotFound { msg: String },
    AlreadyExists { msg: String },
    NotAuthorized { msg: String },
    VotingEnded { msg: String },
    InvalidInput { msg: String },
    VotingNotStarted { msg: String },
    ProposalAlreadyStarted { msg: String },
    InsufficientTokens { msg: String },
}

fn get_current_date() -> (u32, u32, u32) {
    let current_time = (time() / 1_000_000_000) as u64; // Convert nanoseconds to seconds
    let days_since_epoch = current_time / (24 * 60 * 60);
    let mut year = 1970;
    let mut days_remaining = days_since_epoch;

    while days_remaining >= (if is_leap_year(year) { 366 } else { 365 }) {
        days_remaining -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    let mut month = 1;
    while days_remaining >= days_in_month(year, month) as u64 {
        days_remaining -= days_in_month(year, month) as u64;
        month += 1;
    }

    let day = days_remaining as u32 + 1;

    (year as u32, month as u32, day)
}

fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 31,
    }
}

fn date_to_timestamp(year: u32, month: u32, day: u32) -> u64 {
    let mut days = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_month(year, m);
    }
    days += day - 1;
    (days as u64) * 24 * 60 * 60
}

fn parse_date(date_str: &str) -> Result<(u64, u32, u32, u32), Error> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::InvalidInput { msg: "Invalid date format. Use DD-MM-YYYY".to_string() });
    }
    
    let day = parts[0].parse::<u32>().map_err(|_| Error::InvalidInput { msg: "Invalid day".to_string() })?;
    let month = parts[1].parse::<u32>().map_err(|_| Error::InvalidInput { msg: "Invalid month".to_string() })?;
    let year = parts[2].parse::<u32>().map_err(|_| Error::InvalidInput { msg: "Invalid year".to_string() })?;
    
    // Perform basic date validation
    if month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) || year < 2023 {
        return Err(Error::InvalidInput { msg: "Invalid date".to_string() });
    }
    
    let timestamp = date_to_timestamp(year, month, day);
    Ok((timestamp, year, month, day))
}

#[ic_cdk::query]
fn get_proposal(id: u64) -> Result<Proposal, Error> {
    if id == 0 {
        return Err(Error::InvalidInput { msg: "Invalid proposal ID. ID cannot be 0.".to_string() });
    }

    PROPOSALS.with(|proposals| {
        proposals
            .borrow()
            .get(&id)
            .map(|mut p| {
                p.update_status();
                p
            })
            .ok_or(Error::NotFound { msg: "Proposal not found".to_string() })
    })
}

#[ic_cdk::update]
fn create_proposal(payload: ProposalPayload) -> Result<Proposal, Error> {
    let current_time = time() / 1_000_000_000; // Convert nanoseconds to seconds
    let (current_year, current_month, current_day) = get_current_date();

    // Validate dates
    let (start_timestamp, start_year, start_month, start_day) = parse_date(&payload.start_date)?;
    let (end_timestamp, end_year, end_month, end_day) = parse_date(&payload.end_date)?;

    if end_timestamp <= start_timestamp {
        return Err(Error::InvalidInput { msg: "End date must be after start date".to_string() });
    }
    if current_year < start_year || (current_year == start_year && (current_month < start_month || (current_month == start_month && current_day < start_day))) {
        return Err(Error::InvalidInput { msg: "Start date must be in the future".to_string() });
    }

    let mut id_counter = ID_COUNTER.with(|cell| cell.borrow_mut());
    let id = id_counter.increment();

    let proposal = Proposal {
        id,
        title: payload.title,
        description: payload.description,
        creator: PrincipalWrapper(ic_cdk::caller()),
        created_at: current_time,
        start_date: start_timestamp,
        end_date: end_timestamp,
        votes_for: 0,
        votes_against: 0,
        status: ProposalStatus::Pending,
    };

    PROPOSALS.with(|proposals| {
        if proposals.borrow_mut().insert(id, proposal.clone()).is_some() {
            return Err(Error::AlreadyExists { msg: "Proposal with this ID already exists".to_string() });
        }
    });

    Ok(proposal)
}

#[ic_cdk::update]
fn vote_on_proposal(payload: VotePayload) -> Result<String, Error> {
    let caller = PrincipalWrapper(ic_cdk::caller());
    let current_time = time() / 1_000_000_000;

    // Check user tokens
    let mut user_tokens = USER_TOKENS.borrow_mut();
    let user_balance = user_tokens.entry(caller.clone()).or_insert(0);
    if *user_balance < payload.tokens {
        return Err(Error::InsufficientTokens { msg: "Not enough tokens to vote".to_string() });
    }

    PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        let proposal = proposals.get_mut(&payload.proposal_id)
            .ok_or(Error::NotFound { msg: "Proposal not found".to_string() })?;

        proposal.update_status();

        if proposal.status == ProposalStatus::Ended {
            return Err(Error::VotingEnded { msg: "Voting has already ended for this proposal".to_string() });
        }

        if proposal.start_date > current_time {
            return Err(Error::VotingNotStarted { msg: "Voting has not yet started for this proposal".to_string() });
        }

        let vote = Vote {
            user: caller.clone(),
            proposal_id: payload.proposal_id,
            vote_power: payload.tokens,
            is_for: payload.is_for,
        };

        // Update proposal votes
        if payload.is_for {
            proposal.votes_for += payload.tokens;
        } else {
            proposal.votes_against += payload.tokens;
        }

        // Store vote
        VOTES.with(|votes| {
            if votes.borrow_mut().insert((caller.clone(), payload.proposal_id), vote).is_some() {
                return Err(Error::AlreadyExists { msg: "User has already voted on this proposal".to_string() });
            }
        });

        *user_balance -= payload.tokens; // Deduct user tokens after vote
        Ok("Vote recorded successfully".to_string())
    })
}

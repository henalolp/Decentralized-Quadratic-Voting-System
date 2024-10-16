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
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(self.0.as_slice().to_vec())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
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
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Proposal {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Vote {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
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

// #[derive(CandidType, Serialize, Deserialize)]
// struct ProposalPayload {
//     title: String,
//     description: String,
//     duration: u64,
// }

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
    ProposalAlreadyStarted { msg: String }, // Add this line
    InsufficientTokens { msg: String },
}

// Helper function to get current date components
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

// Helper function to check if a year is a leap year
fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// Helper function to get the number of days in a month
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 31
    }
}

// Helper function to convert date to timestamp
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
    
    let (start_timestamp, start_year, start_month, start_day) = parse_date(&payload.start_date)?;
    let (end_timestamp, _, _, _) = parse_date(&payload.end_date)?;

    let start_time = if (start_year, start_month, start_day) == (current_year, current_month, current_day) {
        current_time
    } else if start_timestamp < current_time {
        return Err(Error::InvalidInput { msg: "Start date must be in the future".to_string() });
    } else {
        start_timestamp
    };

    let time_of_day = current_time % (24 * 60 * 60);
    let end_time = end_timestamp + time_of_day;

    if end_time <= start_time {
        return Err(Error::InvalidInput { msg: "End date must be after start date".to_string() });
    }

    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1)
            .expect("Failed to increment counter");
        current_value
    });

    let proposal = Proposal {
        id,
        title: payload.title,
        description: payload.description,
        creator: PrincipalWrapper(ic_cdk::caller()),
        created_at: current_time,
        start_date: start_time,
        end_date: end_time,
        votes_for: 0,
        votes_against: 0,
        status: ProposalStatus::Pending,
    };

    PROPOSALS.with(|proposals| {
        proposals.borrow_mut().insert(id, proposal.clone());
    });

    Ok(proposal)
}

#[ic_cdk::update]
fn vote(payload: VotePayload) -> Result<(), Error> {
    if payload.proposal_id == 0 {
        return Err(Error::InvalidInput { msg: "Invalid proposal ID. ID cannot be 0.".to_string() });
    }

    let caller = ic_cdk::caller();
    let current_time = ic_cdk::api::time() / 1_000_000_000;

    // Check if the user has enough tokens before proceeding
    let user_tokens = USER_TOKENS.with(|tokens| {
        tokens.borrow().get(&PrincipalWrapper(caller)).cloned().unwrap_or(3)
    });

    if user_tokens < payload.tokens {
        return Err(Error::InsufficientTokens { msg: "Not enough tokens to cast vote".to_string() });
    }

    PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        if let Some(proposal) = proposals.get(&payload.proposal_id) {
            let mut proposal = proposal.clone(); // Clone the proposal here
            if current_time < proposal.start_date {
                Err(Error::VotingNotStarted { msg: "Voting has not started yet".to_string() })
            } else if current_time > proposal.end_date {
                Err(Error::VotingEnded { msg: "Voting has ended".to_string() })
            } else {
                // Vote is valid, update the proposal
                if payload.is_for {
                    proposal.votes_for += payload.tokens;
                } else {
                    proposal.votes_against += payload.tokens;
                }
                proposals.insert(payload.proposal_id, proposal);

                // Deduct tokens only after successful vote
                USER_TOKENS.with(|tokens| {
                    let mut tokens = tokens.borrow_mut();
                    let current_tokens = tokens.get(&PrincipalWrapper(caller)).cloned().unwrap_or(3);
                    tokens.insert(PrincipalWrapper(caller), current_tokens - payload.tokens);
                });

                // Record the vote
                VOTES.with(|votes| {
                    votes.borrow_mut().insert(
                        (PrincipalWrapper(caller), payload.proposal_id),
                        Vote {
                            user: PrincipalWrapper(caller),
                            proposal_id: payload.proposal_id,
                            vote_power: payload.tokens,
                            is_for: payload.is_for,
                        },
                    );
                });

                Ok(())
            }
        } else {
            Err(Error::NotFound { msg: "Proposal not found".to_string() })
        }
    })
}

#[ic_cdk::query]
fn get_proposal_results(id: u64) -> Result<(u64, u64), Error> {
    if id == 0 {
        return Err(Error::InvalidInput { msg: "Invalid proposal ID. Input a valid proposal ID".to_string() });
    }

    PROPOSALS.with(|proposals| {
        if let Some(proposal) = proposals.borrow().get(&id) {
            Ok((proposal.votes_for, proposal.votes_against))
        } else {
            Err(Error::NotFound { msg: "Proposal not found".to_string() })
        }
    })
}

#[ic_cdk::query]
fn get_active_proposals() -> Vec<Proposal> {
    let current_time = ic_cdk::api::time() / 1_000_000_000;
    PROPOSALS.with(|proposals| {
        proposals
            .borrow()
            .iter()
            .filter(|(_, proposal)| {
                proposal.start_date <= current_time && proposal.end_date > current_time
            })
            .map(|(_, mut proposal)| {
                proposal.update_status();
                proposal
            })
            .collect()
    })
}

#[ic_cdk::query]
fn get_inactive_proposals() -> Vec<Proposal> {
    let current_time = ic_cdk::api::time() / 1_000_000_000;
    PROPOSALS.with(|proposals| {
        proposals
            .borrow()
            .iter()
            .filter(|(_, p)| p.end_date < current_time)
            .map(|(_, mut p)| {
                p.update_status();
                p
            })
            .collect()
    })
}

#[ic_cdk::query]
fn get_all_proposals() -> Vec<Proposal> {
    PROPOSALS.with(|proposals| {
        proposals
            .borrow()
            .iter()
            .map(|(_, mut p)| {
                p.update_status();
                p
            })
            .collect()
    })
}

#[ic_cdk::update]
fn update_proposal(id: u64, payload: ProposalPayload) -> Result<Proposal, Error> {
    if id == 0 {
        return Err(Error::InvalidInput { msg: "Invalid proposal ID. Input a valid proposal ID".to_string() });
    }

    PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        if let Some(proposal) = proposals.get(&id).map(|p| p.clone()) {
            // Update the proposal fields
            let updated_proposal = Proposal {
                title: payload.title,
                description: payload.description,
                ..proposal
            };
            proposals.insert(id, updated_proposal.clone());
            Ok(updated_proposal)
        } else {
            Err(Error::NotFound { msg: "Proposal not found".to_string() })
        }
    })
}

#[ic_cdk::update]
fn delete_proposal(id: u64) -> Result<(), Error> {
    if id == 0 {
        return Err(Error::InvalidInput { msg: "Invalid proposal ID. Input a valid proposal ID".to_string() });
    }

    PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        if let Some(proposal) = proposals.get(&id) {
            if proposal.creator != PrincipalWrapper(ic_cdk::caller()) {
                return Err(Error::NotAuthorized { msg: "Not authorized to delete this proposal".to_string() });
            }
            if proposal.start_date <= ic_cdk::api::time() / 1_000_000_000 {
                return Err(Error::ProposalAlreadyStarted { msg: "Cannot delete a proposal that has already started".to_string() });
            }
            proposals.remove(&id);
            Ok(())
        } else {
            Err(Error::NotFound { msg: "Proposal not found".to_string() })
        }
    })
}

#[ic_cdk::update]
fn get_vote_tokens(amount: u64) -> Result<(), Error> {
    let caller = PrincipalWrapper(ic_cdk::caller());
    USER_TOKENS.with(|tokens| {
        let mut tokens = tokens.borrow_mut();
        let current_tokens = tokens.entry(caller).or_insert(3);
        *current_tokens += amount;
    });
    Ok(())
}

#[ic_cdk::query]
fn get_user_tokens() -> u64 {
    let caller = PrincipalWrapper(ic_cdk::caller());
    USER_TOKENS.with(|tokens| {
        *tokens.borrow().get(&caller).unwrap_or(&3)
    })
}

fn initialize_user_tokens(caller: &PrincipalWrapper) {
    USER_TOKENS.with(|tokens| {
        tokens.borrow_mut().entry(caller.clone()).or_insert(3);
    });
}

// Candid export
ic_cdk::export_candid!();

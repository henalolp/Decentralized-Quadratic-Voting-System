use chrono::{NaiveDate, Datelike};
use std::collections::HashMap;
use ic_cdk::api::time;

#[derive(Clone)]
pub struct Proposal {
    pub start_date: u64,
    pub end_date: u64,
    pub title: String,
    // other fields...
}

#[derive(Debug)]
pub enum ProposalStatus {
    Active,
    Ended,
    // other statuses...
}

pub struct Error {
    pub msg: String,
}

thread_local! {
    static PROPOSALS: RefCell<HashMap<u64, Proposal>> = RefCell::new(HashMap::new());
    static USER_TOKENS: RefCell<HashMap<PrincipalWrapper, u64>> = RefCell::new(HashMap::new());
}

const DEFAULT_USER_TOKENS: u64 = 3;

impl Proposal {
    pub fn update_status(&mut self) {
        let current_time = time() / 1_000_000_000;
        if current_time < self.start_date {
            // Status logic
        } else if current_time > self.end_date {
            // Status logic
        }
    }
}

pub fn parse_date(date_str: &str) -> Result<(u64, u32, u32, u32), Error> {
    let date = NaiveDate::parse_from_str(date_str, "%d-%m-%Y").map_err(|_| Error { msg: "Invalid date".to_string() })?;
    let timestamp = date.timestamp() as u64;
    Ok((timestamp, date.year() as u32, date.month(), date.day()))
}

pub fn filter_proposals(status: ProposalStatus) -> Vec<Proposal> {
    let current_time = time() / 1_000_000_000;
    PROPOSALS.with(|proposals| {
        proposals.borrow()
            .iter()
            .filter_map(|(_, proposal)| {
                match status {
                    ProposalStatus::Active if proposal.start_date <= current_time && proposal.end_date > current_time => {
                        proposal.update_status();
                        Some(proposal.clone())
                    },
                    ProposalStatus::Ended if proposal.end_date < current_time => {
                        proposal.update_status();
                        Some(proposal.clone())
                    },
                    _ => None,
                }
            })
            .collect()
    })
}

pub fn initialize_user_tokens(caller: &PrincipalWrapper) {
    USER_TOKENS.with(|tokens| {
        if !tokens.borrow().contains_key(caller) {
            tokens.borrow_mut().insert(caller.clone(), DEFAULT_USER_TOKENS);
        }
    });
}

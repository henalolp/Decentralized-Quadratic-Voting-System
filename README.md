# Decentralized Quadratic Voting System

## Overview

This project is a decentralized quadratic voting system on the Internet Computer Protocol (ICP) using Rust. Users can create, vote on, and manage proposals in a secure and decentralized manner. Quadratic voting allows users to cast more votes with fewer tokens, promoting more thoughtful and considered voting. For example, a user with 100 tokens can cast 100 votes, but if they want to cast 1000 votes, they need to have 1000 tokens.

## Features

- Create proposals with customizable start and end dates
- Vote on proposals using a token-based system
- Query active proposals and results
- Update and delete proposals (creator only)
- Secure user authentication via ICP's principal system

### Requirements
* Node.js and npm (latest version)
* rustc 1.64 or higher
```bash
$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
$ source "$HOME/.cargo/env"
```
* rust wasm32-unknown-unknown target
```bash
$ rustup target add wasm32-unknown-unknown
```
* candid-extractor
```bash
$ cargo install candid-extractor
```
* DFX (latest version) install `dfx`
```bash
$ DFX_VERSION=0.15.0 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"
$ echo 'export PATH="$PATH:$HOME/bin"' >> "$HOME/.bashrc"
$ source ~/.bashrc
$ dfx start --background
```

## Setup

1. Clone the repository:
   ```
   git clone https://github.com/your-username/icp-rust-boilerplate.git
   cd icp-rust-boilerplate
   ```

2. Install dependencies:
   ```
   npm install
   ```

3. Start the local ICP network:
   ```
   dfx start --background
   ```

4. Deploy the canister:
   ```
   npm run gen-deploy
   ```

### Use the icp_rust_boilerplate_backend canister url to interact with the canister in your browser via the Candid UI.

## Project Structure

- `src/icp_rust_boilerplate_backend/`: Rust backend code
- `src/declarations/`: Auto-generated type declarations
- `Cargo.toml`: Rust dependencies and configuration
- `dfx.json`: DFX configuration file

## Key Components

### Proposal Structure

Proposals are stored in a stable-structures database and are indexed by their ID. Each proposal has the following fields:

- `id`: Unique identifier for the proposal
- `title`: Title of the proposal
- `description`: Description of the proposal
- `start_date`: Start date of the proposal (DD-MM-YYYY)
- `end_date`: End date of the proposal (DD-MM-YYYY)
- `status`: Status of the proposal (active, closed, or deleted)
- `creator`: Principal ID of the creator
- `votes`: Votes for the proposal

### Voting System

Voting is token-based, with each user having a certain number of tokens they can allocate to proposals. The voting power is proportional to the number of tokens allocated.
Initially you have 3 vote tokens, you could get some vote tokens by using the get_vote_tokens method and providing the number of vote tokens needed. For testing purpose, the tokens will be provided without actual cost.
In a real platform, this could be setup such that users will be billed to get extra tokens to vote.


### Main Functions

- `create_proposal`: Create a new proposal
- `get_proposal`: Retrieve a specific proposal
- `get_active_proposals`: List all active proposals
- `vote`: Cast a vote on a proposal
- `get_proposal_results`: Get voting results for a proposal
- `update_proposal`: Modify an existing proposal (creator only)
- `delete_proposal`: Remove a proposal (creator only)

## Usage Examples

Here are some examples using the `dfx` command-line tool:

### Create a Proposal
```bash
dfx canister call icp_rust_boilerplate_backend create_proposal "(
  'My Proposal Title',
  'This is a description of my proposal',
  '01-01-2024',
  '01-02-2024'
)
```
### Delete a proposal (To delete a proposal with ID 1)
```bash
dfx canister call icp_rust_boilerplate_backend delete_proposal "(1)"
```
### Get active proposals (To get all active proposals)
```bash
dfx canister call icp_rust_boilerplate_backend get_active_proposals "()"
```
### Get all proposals (To get all proposals)
```bash
dfx canister call icp_rust_boilerplate_backend get_all_proposals "()"
```
### Get inactive proposals (To get all inactive proposals)
```bash
dfx canister call icp_rust_boilerplate_backend get_inactive_proposals "()"
```

### Get a proposal (To get a proposal with ID 1)
```bash
dfx canister call icp_rust_boilerplate_backend get_proposal "(1)"
```
### Get proposal result (To get the results of a proposal with ID 1)
```bash
dfx canister call icp_rust_boilerplate_backend get_proposal_results "(1)"
```
### and more...
### For ease interaction with the canister, use the Candid UI

## Contributions

I welcome contributions to this project! If you’d like to get involved, please follow these steps:

1. Fork the repository.
2. Create a new branch for your changes.
3. Make your modifications.
4. Submit a pull request.

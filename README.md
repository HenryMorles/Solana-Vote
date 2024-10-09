# Solana Voting Program

This is a Rust-based voting program built on the Solana blockchain. The program allows creators to set up voting sessions with multiple options, restrict voting to allowed participants, delegate votes, and securely track results.

## Features

- **Create Voting Sessions**: Users can create votes with different options.
- **Add and Remove Allowed Voters**: Only authorized participants can vote, and the creator can manage the list of allowed voters.
- **Cast Votes**: Voters can cast their vote on the specified options if they have permissions.
- **Delegate Votes**: Voters can delegate their vote to another allowed voter.
- **Close Votes**: The creator can close the voting session to prevent any further changes.
- **View Results**: View the results of a vote if they are not closed to public viewing.
- **Testing Suite**: Comprehensive unit tests to verify the core functionality.

## Table of Contents

- [Getting Started](#getting-started)
- [Building the Program](#building-the-program)
- [Testing](#testing)
- [Usage](#usage)
  - [Creating a Vote](#creating-a-vote)
  - [Adding Allowed Voters](#adding-allowed-voters)
  - [Voting](#voting)
  - [Delegating Votes](#delegating-votes)
  - [Closing a Vote](#closing-a-vote)
  - [Viewing Results](#viewing-results)
- [License](#license)

## Getting Started

To start working with the Solana Voting Program, you need to have Rust and Solana development tools installed.

### Prerequisites

- **Rust**: Install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).
- **Solana CLI**: Install Solana CLI tools from [here](https://docs.solana.com/cli/install-solana-cli-tools).

### Installation

Clone the repository to your local machine:

```bash
git clone https://github.com/HenryMorles/Solana-Vote.git
cd solana-voting-program
```

## Building the Program

To build the program, use the following command:

```bash
cargo build-bpf
```

This will generate the necessary binaries for deploying to the Solana blockchain.

## Testing

The project comes with a comprehensive set of unit tests that cover the core functionality of the voting program. To run the tests, execute:

```bash
cargo test
```

Make sure all tests pass before deploying the program.

## Usage

### Creating a Vote

To create a vote, call the `create_vote` method with the title, voting options, visibility status for the results, and the account information of the creator.

Example:

```rust
let vote_id = test_voting.add_vote(
    "Test Vote".to_string(),
    vec!["Option 1".to_string(), "Option 2".to_string()],
    false, // Results are open
    creator_pubkey,
);
```

### Adding Allowed Voters

Only allowed voters can cast their vote. The creator can add voters using the `add_allowed_voter` method.

Example:

```rust
test_voting.voting.add_allowed_voter(vote_id, voter_pubkey, &[creator_account_info]);
```

### Voting

To cast a vote, an allowed voter uses the `vote` method, specifying the option index they want to vote for.

Example:

```rust
test_voting.voting.vote(vote_id, &[voter_account_info], option_index);
```

### Delegating Votes

An allowed voter can delegate their vote to another participant using the `delegate_vote` method.

Example:

```rust
test_voting.voting.delegate_vote(vote_id, &delegate_pubkey, &[delegator_account_info]);
```

### Closing a Vote

To close a vote and prevent further changes, the creator can use the `close_vote` method.

Example:

```rust
test_voting.voting.close_vote(vote_id, &[creator_account_info]);
```

### Viewing Results

To view the results of a vote, use the `get_results` method. If the results are set to be private, only allowed voters can view them.

Example:

```rust
let results = test_voting.voting.get_results(vote_id, &[viewer_account_info])?;
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

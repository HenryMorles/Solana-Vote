use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use solana_program::account_info::AccountInfo;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct VoterInfo {
    pub votes_left: u32,         // Number of remaining votes
    pub delegate: Option<Pubkey>, // Delegate if any
}

#[derive(Debug)]
pub struct Vote {
    id: u32,
    title: String,
    options: Vec<String>,
    votes: HashMap<String, u32>,
    creator: Pubkey,
    allowed_voters: HashMap<Pubkey, VoterInfo>, // Stores information about allowed voters
    is_close_vote_results: bool,
    is_vote_open: bool
}

impl Vote {
    // Method to get voting options
    fn get_options(&self) -> &Vec<String> {
        &self.options
    }

    fn add_allowed_voter(&mut self, voter: Pubkey, caller: &Pubkey) -> Result<(), ProgramError>{
        if *caller != self.creator {
            return Err(ProgramError::InvalidArgument); // Return error if not the creator
        }

        // Check if the voting is closed
        if !self.is_vote_open {
            return Err(ProgramError::InvalidArgument); // Return error if voting is closed
        }

        let new_voter = VoterInfo {
            votes_left: 1,            // Initialize with 1 vote
            delegate: None,           // Empty delegate
        };

        self.allowed_voters.insert(voter, new_voter); // Initialize new voter

        Ok(())
    }

    fn remove_allowed_voter(&mut self, voter: &Pubkey, caller: &Pubkey) -> Result<(), ProgramError> {
        // Check if the calling address is the creator of the vote
        if *caller != self.creator {
            return Err(ProgramError::InvalidArgument); // Return error if not the creator
        }

        // Check if the voting is closed
        if !self.is_vote_open {
            return Err(ProgramError::InvalidArgument); // Return error if voting is closed
        }

        // Remove the voter from the list if they exist
        if self.allowed_voters.remove(voter).is_some() {
            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the voter is not found
        }
    }

    fn is_voter_allowed(&self, voter: &Pubkey) -> bool {
        self.allowed_voters.contains_key(voter)
    }

    fn vote(&mut self, voter: &Pubkey, option_index: usize) -> Result<(), ProgramError> {
        // Check if the voter is in the allowed list
        if !self.is_voter_allowed(voter) {
            return Err(ProgramError::InvalidArgument); // Return error if voter is not allowed
        }

        // Check if the voting is closed
        if !self.is_vote_open {
            return Err(ProgramError::InvalidArgument); // Return error if voting is closed
        }

        if let Some(voter_info) = self.allowed_voters.get_mut(voter) {
            // Check if the voter still has votes left
            if voter_info.votes_left <= 0 {
                return Err(ProgramError::InvalidArgument); // Return error if the voter has exhausted their votes
            }

            // Check if the selected option index is correct
            if option_index >= self.options.len() {
                return Err(ProgramError::InvalidArgument); // Return error if index is out of range
            }

            // Increase the number of votes for the selected option
            let option_key = self.options[option_index].clone();
            let count = self.votes.entry(option_key).or_insert(0);
            *count += 1; // Increase the vote count

            // Decrease the remaining votes
            voter_info.votes_left -= 1;

            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the voter is not found
        }
    }

    fn delegate_vote(&mut self, delegate: &Pubkey, delegator: &Pubkey) -> Result<(), ProgramError> {
        // Check if the delegator is allowed
        if let Some(voter_info) = self.allowed_voters.get(delegator).cloned() {
            // Check if the voting is closed
            if !self.is_vote_open {
                return Err(ProgramError::InvalidArgument); // Return error if voting is closed
            }

            if voter_info.votes_left > 0 {
                // Decrease the number of votes for the delegator
                let mut updated_voter_info = voter_info;
                updated_voter_info.votes_left -= 1;

                // Get or create an entry for the delegate
                let entry = self.allowed_voters.entry(*delegate).or_insert(VoterInfo {
                    votes_left: 0,
                    delegate: None,
                });

                // Increase the number of votes for the delegate
                entry.votes_left += 1;

                // Set the delegate
                updated_voter_info.delegate = Some(*delegate);
                self.allowed_voters.insert(*delegator, updated_voter_info); // Update the voter's information

                Ok(())
            } else {
                Err(ProgramError::InvalidArgument) // No available votes
            }
        } else {
            Err(ProgramError::InvalidArgument) // Delegator is not allowed
        }
    }
}

pub struct Voting {
    pub votes: HashMap<u32, Vote>, // List of votes
    current_id: u32,
}

impl Voting {

    pub fn create_vote(&mut self, title: String, options: Vec<String>, is_close_vote_results: bool, accounts: &[AccountInfo]) -> Result<u32, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let creator = accounts[0].key;
        let is_vote_open = true;

        let vote = Vote {
            id: self.current_id,
            title,
            options,
            votes: HashMap::new(), // Initialize an empty map for votes
            creator: *creator,
            allowed_voters: HashMap::new(), // Initialize an empty map for allowed voters
            is_close_vote_results,
            is_vote_open
        };
        self.votes.insert(self.current_id, vote); // Add the vote to the list
        self.current_id += 1; // Increment the identifier for the next vote

        Ok(self.current_id - 1)
    }

    pub fn vote(&mut self, vote_id: u32, accounts: &[AccountInfo], option_index: usize) -> Result<(), ProgramError> {
        // Check if the provided vote ID is valid
        if !self.votes.contains_key(&vote_id) {
            return Err(ProgramError::InvalidArgument); // Return error if the ID does not exist
        }

        // Get the vote by ID
        let vote = self.votes.get_mut(&vote_id).unwrap(); // Safely extract the vote as we already checked for existence

        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let voter = accounts[0].key;

        // Call the voting method
        vote.vote(voter, option_index)
    }

    pub fn close_vote(&mut self, vote_id: u32, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument);
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            if vote.creator != *caller {
                return Err(ProgramError::InvalidArgument); // Only the creator can close the vote
            }
            vote.is_vote_open = false; // Close the vote
            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Vote not found
        }
    }

    pub fn get_results(&self, vote_id: u32, accounts: &[AccountInfo]) -> Result<HashMap<String, u32>, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let caller = accounts[0].key;

        // Extract the vote by ID
        let vote = self.votes.get(&vote_id).ok_or(ProgramError::InvalidArgument)?;

        // Check if the results of the vote are closed
        if vote.is_close_vote_results {
            // Check if the voter is allowed
            if !vote.is_voter_allowed(caller) {
                return Err(ProgramError::InvalidArgument); // Return error if the voter is not allowed
            }
        }

        // Return the voting results
        Ok(vote.votes.clone())
    }

    pub fn add_allowed_voter(&mut self, vote_id: u32, voter: Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            vote.add_allowed_voter(voter, caller)
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the vote does not exist
        }
    }

    pub fn remove_allowed_voter(&mut self, vote_id: u32, voter: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            vote.remove_allowed_voter(voter, caller)
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the vote does not exist
        }
    }

    pub fn is_voter_allowed(&self, vote_id: u32, voter: &Pubkey) -> Result<bool, ProgramError> {
        if let Some(vote) = self.votes.get(&vote_id) {
            Ok(vote.is_voter_allowed(voter))
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the vote does not exist
        }
    }

    pub fn delegate_vote(&mut self, vote_id: u32, delegate: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        // Check if the vote with the given ID exists
        let vote = self.votes.get_mut(&vote_id).ok_or(ProgramError::InvalidArgument)?;

        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Return error if no accounts are provided
        }

        let delegator = accounts[0].key;

        // Call the delegate_vote method of the vote
        vote.delegate_vote(delegate, delegator)
    }

    pub fn get_options(&mut self, vote_id: u32) -> Result<&Vec<String>, ProgramError> {
        if let Some(vote) = self.votes.get(&vote_id) {
            Ok(vote.get_options())
        } else {
            Err(ProgramError::InvalidArgument) // Return error if the vote does not exist
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;
    use std::collections::HashMap;

    struct TestVoting {
        voting: Voting,
        lamports: u64,
        data: Vec<u8>,
        owner: Pubkey,
    }

    impl TestVoting {
        fn new() -> Self {
            Self {
                voting: Voting {
                    votes: HashMap::new(),
                    current_id: 0,
                },
                lamports: 0,
                data: vec![],
                owner: Pubkey::new_unique(),
            }
        }

        fn add_vote(&mut self, title: String, options: Vec<String>, is_close_vote_results: bool, creator: Pubkey) -> u32 {
            let is_signer = true;
            let is_writable = false;
            let executable = false;

            // Create AccountInfo for the vote creator
            let account_info = AccountInfo::new(
                &creator,
                is_signer,
                is_writable,
                &mut self.lamports,
                &mut self.data,
                &self.owner,
                executable,
                0,
            );

            match self.voting.create_vote(title, options, is_close_vote_results, &[account_info]) {
                Ok(vote_id) => vote_id,
                Err(err) => {
                    panic!("Failed to create vote: {:?}", err);
                }
            }
        }
    }

    #[test]
    fn test_create_vote() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string(), "Option 2".to_string()], false, creator);

        assert_eq!(test_voting.voting.votes.len(), 1);
        let vote = test_voting.voting.votes.get(&0).unwrap();
        assert_eq!(vote.title, "Test Vote");
        assert_eq!(vote.options.len(), 2);
    }

    #[test]
    fn test_add_allowed_voter() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        let vote = test_voting.voting.votes.get(&0).unwrap();
        assert!(vote.is_voter_allowed(&voter1));
    }

    #[test]
    fn test_vote() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string(), "Option 2".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        let account_info_voter1 = AccountInfo::new(
            &voter1,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.vote(0, &[account_info_voter1], 0).is_ok());

        let vote = test_voting.voting.votes.get_mut(&0).unwrap();
        assert_eq!(*vote.votes.get("Option 1").unwrap(), 1);
    }

    #[test]
    fn test_vote_not_allowed() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();
        let voter2 = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        let account_info_voter2 = AccountInfo::new(
            &voter2,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.vote(0, &[account_info_voter2], 0).is_err()); // Voter is not allowed
    }

    #[test]
    fn test_vote_no_votes_left() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).unwrap();

        // Set that voter1 has no votes left
        let new_voter = VoterInfo {
            votes_left: 0,
            delegate: None,
        };
        test_voting.voting.votes.get_mut(&0).unwrap().allowed_voters.insert(voter1, new_voter);

        let account_info_voter1 = AccountInfo::new(
            &voter1,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.vote(0, &[account_info_voter1], 0).is_err()); // No votes left for voting
    }

    #[test]
    fn test_remove_allowed_voter() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info.clone()]).is_ok());

        // Remove the allowed voter
        assert!(test_voting.voting.remove_allowed_voter(0, &voter1, &[account_info]).is_ok());
        let vote = test_voting.voting.votes.get(&0).unwrap();
        assert!(!vote.is_voter_allowed(&voter1)); // Check that the voter has been removed
    }

    #[test]
    fn test_remove_allowed_voter_not_creator() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();
        let non_creator = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        let non_creator_info = AccountInfo::new(
            &non_creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        // Attempt to remove an allowed voter not as the creator
        assert!(test_voting.voting.remove_allowed_voter(0, &voter1, &[non_creator_info]).is_err());
    }

    #[test]
    fn test_delegate_vote() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        // Set that voter1 has 1 vote
        let new_voter = VoterInfo {
            votes_left: 1,
            delegate: None,
        };
        test_voting.voting.votes.get_mut(&0).unwrap().allowed_voters.insert(voter1, new_voter);

        let account_info_voter1 = AccountInfo::new(
            &voter1,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        // Perform the vote delegation
        let result = test_voting.voting.delegate_vote(0, &delegate, &[account_info_voter1]);

        assert!(result.is_ok());

        if let Some(voter_info) = test_voting.voting.votes.get_mut(&0).unwrap().allowed_voters.get(&voter1) {
            assert_eq!(voter_info.votes_left, 0);
            assert_eq!(voter_info.delegate, Some(delegate));
        } else {
            panic!("Voter1 information not found.");
        }

        if let Some(delegate_info) = test_voting.voting.votes.get_mut(&0).unwrap().allowed_voters.get(&delegate) {
            assert_eq!(delegate_info.votes_left, 1);
        } else {
            panic!("Delegate information not found.");
        }
    }

    #[test]
    fn test_delegate_vote_not_allowed() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let non_allowed_voter = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        let account_info_non_allowed = AccountInfo::new(
            &non_allowed_voter,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        // Attempt to delegate vote to a non-allowed voter
        assert!(test_voting.voting.delegate_vote(0, &delegate, &[account_info_non_allowed]).is_err());
    }

    #[test]
    fn test_delegate_vote_no_votes_left() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(
            &creator,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info]).is_ok());

        // Set that voter1 has no votes
        let new_voter = VoterInfo {
            votes_left: 0,
            delegate: None,
        };
        test_voting.voting.votes.get_mut(&0).unwrap().allowed_voters.insert(voter1, new_voter);

        let account_info_voter1 = AccountInfo::new(
            &voter1,
            is_signer,
            is_writable,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            executable,
            0,
        );

        // Check that delegation fails as voter1 has no votes
        assert!(test_voting.voting.delegate_vote(0, &delegate, &[account_info_voter1]).is_err());
    }

    #[test]
    fn test_vote_nonexistent() {
        let mut test_voting = TestVoting::new();
        let voter1 = Pubkey::new_unique();

        let account_info = AccountInfo::new(
            &voter1,
            true,
            false,
            &mut test_voting.lamports,
            &mut test_voting.data,
            &test_voting.owner,
            false,
            0,
        );

        // Attempt to vote on a non-existent voting
        assert!(test_voting.voting.vote(999, &[account_info], 0).is_err());
    }

    #[test]
    fn test_vote_after_closing() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        // Create a vote
        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string(), "Option 2".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Create AccountInfo for the vote creator
        let account_info = AccountInfo::new(&creator, is_signer, is_writable, &mut test_voting.lamports, &mut test_voting.data, &test_voting.owner, executable, 0, );

        // Add allowed voter
        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info.clone()]).is_ok());

        // Close the vote
        assert!(test_voting.voting.close_vote(0, &[account_info.clone()]).is_ok());

        // Now try to vote after the voting is closed
        let account_info_voter1 = AccountInfo::new(&voter1, is_signer, is_writable, &mut test_voting.lamports, &mut test_voting.data, &test_voting.owner, executable, 0, );

        // Check that voting does not pass as it is closed
        assert!(test_voting.voting.vote(0, &[account_info_voter1], 0).is_err());
    }
}

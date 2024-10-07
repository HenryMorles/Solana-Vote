use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use solana_program::account_info::AccountInfo;
use std::collections::HashMap;


#[derive(Debug, Clone)]
struct VoterInfo {
    pub votes_left: u32,         // Количество оставшихся голосов
    pub delegate: Option<Pubkey>, // Делегат, если есть
}

#[derive(Debug)]
pub struct Vote {
    id: u32,
    title: String,
    options: Vec<String>,
    votes: HashMap<String, u32>,
    creator: Pubkey,
    allowed_voters: HashMap<Pubkey, VoterInfo>, // Хранит информацию о разрешённых голосующих
    is_close_vote_results: bool,
    is_vote_open: bool
}

impl Vote {
    // Метод для получения вариантов голосования
    fn get_options(&self) -> &Vec<String> {
        &self.options
    }

    fn add_allowed_voter(&mut self, voter: Pubkey, caller: &Pubkey) -> Result<(), ProgramError>{
        if *caller != self.creator {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если это не создатель
        }

        // Проверяем, не закрыто ли голосование
        if !self.is_vote_open  {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосование закрыто
        }

        let new_voter = VoterInfo {
            votes_left: 1,            // Инициализируем с 1 голосом
            delegate: None,           // Пустой делегат
        };

        self.allowed_voters.insert(voter, new_voter); // Инициализируем нового голосующего

        Ok(())
    }

    fn remove_allowed_voter(&mut self, voter: &Pubkey, caller: &Pubkey) -> Result<(), ProgramError> {
        // Проверяем, что вызывающий адрес - это создатель голосования
        if *caller != self.creator {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если это не создатель
        }

        // Проверяем, не закрыто ли голосование
        if !self.is_vote_open  {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосование закрыто
        }

        // Удаляем голосующего из списка, если он там есть
        if self.allowed_voters.remove(voter).is_some() {
            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосующий не найден
        }
    }

    fn is_voter_allowed(&self, voter: &Pubkey) -> bool {
        self.allowed_voters.contains_key(voter)
    }

    fn vote(&mut self, voter: &Pubkey, option_index: usize) -> Result<(), ProgramError> {
        // Проверяем, что голосующий в списке разрешённых
        if !self.is_voter_allowed(voter) {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосующий не разрешён
        }

        // Проверяем, не закрыто ли голосование
        if !self.is_vote_open  {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосование закрыто
        }

        if let Some(voter_info) = self.allowed_voters.get_mut(voter) {
            // Проверяем, что голосующий ещё может голосовать
            if voter_info.votes_left <= 0 {
                return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосующий исчерпал свои голоса
            }

            // Проверяем, что выбранный индекс варианта корректен
            if option_index >= self.options.len() {
                return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если индекс вне диапазона
            }

            // Увеличиваем количество голосов для выбранного варианта
            let option_key = self.options[option_index].clone();
            let count = self.votes.entry(option_key).or_insert(0);
            *count += 1; // Увеличиваем счетчик голосов

            // Уменьшаем количество оставшихся голосов
            voter_info.votes_left -= 1;

            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосующий не найден
        }
    }

    fn delegate_vote(&mut self, delegate: &Pubkey, delegator: &Pubkey) -> Result<(), ProgramError> {
        // Проверяем, что делегатор разрешён
        if let Some(voter_info) = self.allowed_voters.get(delegator).cloned() {
            // Проверяем, не закрыто ли голосование
            if !self.is_vote_open  {
                return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосование закрыто
            }

            if voter_info.votes_left > 0 {
                // Уменьшаем количество голосов у делегатора
                let mut updated_voter_info = voter_info;
                updated_voter_info.votes_left -= 1;

                // Получаем или создаем запись для делегата
                let entry = self.allowed_voters.entry(*delegate).or_insert(VoterInfo {
                    votes_left: 0,
                    delegate: None,
                });

                // Увеличиваем количество голосов у делегата
                entry.votes_left += 1;

                // Устанавливаем делегата
                updated_voter_info.delegate = Some(*delegate);
                self.allowed_voters.insert(*delegator, updated_voter_info); // Обновляем информацию о голосующем

                Ok(())
            } else {
                Err(ProgramError::InvalidArgument) // Нет доступных голосов
            }
        } else {
            Err(ProgramError::InvalidArgument) // Делегатор не разрешён
        }
    }
}

pub struct Voting {
    pub votes: HashMap<u32, Vote>, // Список голосований
    current_id: u32,
}

impl Voting {

    pub fn create_vote(&mut self, title: String, options: Vec<String>, is_close_vote_results: bool, accounts: &[AccountInfo]) -> Result<u32, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let creator = accounts[0].key;
        let is_vote_open = true;

        let vote = Vote {
            id: self.current_id,
            title,
            options,
            votes: HashMap::new(), // Инициализируем пустую карту для голосов
            creator: *creator,
            allowed_voters: HashMap::new(), // Инициализируем пустую карту для разрешённых голосующих
            is_close_vote_results,
            is_vote_open
        };
        self.votes.insert(self.current_id, vote); // Добавляем голосование в список
        self.current_id += 1; // Увеличиваем идентификатор для следующего голосования

        Ok(self.current_id - 1)
    }

    pub fn vote(&mut self, vote_id: u32, accounts: &[AccountInfo], option_index: usize) -> Result<(), ProgramError> {
        // Проверяем, что указанный идентификатор голосования корректен
        if !self.votes.contains_key(&vote_id) {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если идентификатор не существует
        }

        // Получаем голосование по идентификатору
        let vote = self.votes.get_mut(&vote_id).unwrap(); // безопасно извлекаем голосование, так как мы уже проверили наличие

        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let voter = accounts[0].key;

        // Вызываем метод голосования
        vote.vote(voter, option_index)
    }

    pub fn close_vote(&mut self, vote_id: u32, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument);
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            if vote.creator != *caller {
                return Err(ProgramError::InvalidArgument); // Только создатель может закрыть голосование
            }
            vote.is_vote_open = false; // Закрываем голосование
            Ok(())
        } else {
            Err(ProgramError::InvalidArgument) // Голосование не найдено
        }
    }

    pub fn get_results(&self, vote_id: u32, accounts: &[AccountInfo]) -> Result<HashMap<String, u32>, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let caller = accounts[0].key;

        // Извлекаем голосование по идентификатору
        let vote = self.votes.get(&vote_id).ok_or(ProgramError::InvalidArgument)?;

        // Проверяем, закрыты ли результаты голосования
        if vote.is_close_vote_results {
            // Проверяем, что голосующий разрешён
            if !vote.is_voter_allowed(caller) {
                return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если голосующий не разрешён
            }
        }

        // Возвращаем результаты голосования
        Ok(vote.votes.clone())
    }

    pub fn add_allowed_voter(&mut self, vote_id: u32, voter: Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            vote.add_allowed_voter(voter, caller)
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосования не существует
        }
    }

    pub fn remove_allowed_voter(&mut self, vote_id: u32, voter: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let caller = accounts[0].key;

        if let Some(vote) = self.votes.get_mut(&vote_id) {
            vote.remove_allowed_voter(voter, caller)
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосования не существует
        }
    }

    pub fn is_voter_allowed(&self, vote_id: u32, voter: &Pubkey) -> Result<bool, ProgramError> {
        if let Some(vote) = self.votes.get(&vote_id) {
            Ok(vote.is_voter_allowed(voter))
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосования не существует
        }
    }

    pub fn delegate_vote(&mut self, vote_id: u32, delegate: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        // Проверяем, что голосование с указанным идентификатором существует
        let vote = self.votes.get_mut(&vote_id).ok_or(ProgramError::InvalidArgument)?;

        if accounts.is_empty() {
            return Err(ProgramError::InvalidArgument); // Возвращаем ошибку, если нет аккаунтов
        }

        let delegator = accounts[0].key;

        // Вызов метода delegate_vote у голосования
        vote.delegate_vote(delegate, delegator)
    }

    pub fn get_options(&mut self, vote_id: u32) -> Result<&Vec<String>, ProgramError> {
        if let Some(vote) = self.votes.get(&vote_id) {
            Ok(vote.get_options())
        } else {
            Err(ProgramError::InvalidArgument) // Возвращаем ошибку, если голосования не существует
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

            // Создаем AccountInfo для создателя голосования
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

            match self.voting.create_vote(title, options,is_close_vote_results, &[account_info]) {
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

        // Создаем AccountInfo для создателя голосования
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

        // Создаем AccountInfo для создателя голосования
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

        // Создаем AccountInfo для создателя голосования
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

        assert!(test_voting.voting.vote(0, &[account_info_voter2], 0).is_err()); // Голосующий не разрешён
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

        // Создаем AccountInfo для создателя голосования
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

        // Устанавливаем, что у voter1 нет голосов
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

        assert!(test_voting.voting.vote(0, &[account_info_voter1], 0).is_err()); // Нет голосов для голосования
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

        // Создаем AccountInfo для создателя голосования
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

        // Удаляем разрешенного голосующего
        assert!(test_voting.voting.remove_allowed_voter(0, &voter1, &[account_info]).is_ok());
        let vote = test_voting.voting.votes.get(&0).unwrap();
        assert!(!vote.is_voter_allowed(&voter1)); // Проверяем, что голосующий удалён
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

        // Создаем AccountInfo для создателя голосования
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

        // Пытаемся удалить разрешённого голосующего не создателем
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

        // Создаем AccountInfo для создателя голосования
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

        // Устанавливаем, что у voter1 есть 1 голос
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

        // Выполняем делегирование голосов
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

        // Создаем AccountInfo для создателя голосования
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

        // Пытаемся делегировать голос неразрешённому голосующему
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

        // Создаем AccountInfo для создателя голосования
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

        // Устанавливаем, что у voter1 нет голосов
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

        // Проверяем, что делегирование не проходит, так как у voter1 нет голосов
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

        // Пытаемся голосовать по несуществующему голосованию
        assert!(test_voting.voting.vote(999, &[account_info], 0).is_err());
    }

    #[test]
    fn test_vote_after_closing() {
        let mut test_voting = TestVoting::new();
        let creator = Pubkey::new_unique();
        let voter1 = Pubkey::new_unique();

        // Создаем голосование
        test_voting.add_vote("Test Vote".to_string(), vec!["Option 1".to_string(), "Option 2".to_string()], false, creator);

        let is_signer = true;
        let is_writable = false;
        let executable = false;

        // Создаем AccountInfo для создателя голосования
        let account_info = AccountInfo::new(&creator, is_signer, is_writable, &mut test_voting.lamports, &mut test_voting.data, &test_voting.owner, executable, 0, );

        // Добавляем разрешенного голосующего
        assert!(test_voting.voting.add_allowed_voter(0, voter1, &[account_info.clone()]).is_ok());

        // Закрываем голосование
        assert!(test_voting.voting.close_vote(0, &[account_info.clone()]).is_ok());

        // Теперь пытаемся проголосовать после закрытия голосования
        let account_info_voter1 = AccountInfo::new(&voter1, is_signer, is_writable, &mut test_voting.lamports, &mut test_voting.data, &test_voting.owner, executable, 0, );

        // Проверяем, что голосование не проходит, так как голосование закрыто
        assert!(test_voting.voting.vote(0, &[account_info_voter1], 0).is_err());
    }
}
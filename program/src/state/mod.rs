pub mod game;
pub mod stats;
pub mod user_account;
pub mod vrf_result;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError};
use std::fmt::Display;

use crate::error::BettingError;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum StateAccountType {
    Uninitialized,
    Stats,
    UserAccount,
    Vrf,
    Game,
}

impl Display for StateAccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateAccountType::Uninitialized => write!(f, "Uninitialized"),
            StateAccountType::UserAccount => write!(f, "UserAccount"),
            StateAccountType::Stats => write!(f, "Stats"),
            StateAccountType::Vrf => write!(f, "VrfResult"),
            StateAccountType::Game => write!(f, "Game"),
        }
    }
}

pub trait BettingAccount: BorshDeserialize {
    const ACCOUNT_TYPE: StateAccountType;
    fn try_from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
        if account_info.owner != &crate::ID {
            msg!("Expect account {} to be owned by this program", account_info.key);
            Err(ProgramError::from(BettingError::WrongAccountOwner))
        } else if account_info.data_is_empty() {
            msg!("Expect account {} to be initialized", account_info.key);
            Err(ProgramError::UninitializedAccount)
        } else {
            let data = account_info.data.borrow();
            if data[0] == StateAccountType::Uninitialized as u8 {
                msg!("Expect account {} to be initialized", account_info.key);
                Err(ProgramError::UninitializedAccount)
            } else if data[0] != Self::ACCOUNT_TYPE as u8 {
                msg!("Expect account {} to be of type {}", account_info.key, Self::ACCOUNT_TYPE);
                Err(ProgramError::InvalidAccountData)
            } else {
                match Self::try_from_slice(&data) {
                    Ok(state) => Ok(state),
                    Err(_) => {
                        msg!("Error deserializing account {}");
                        Err(ProgramError::InvalidAccountData)
                    }
                }
            }
        }
    }
}

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::error::BettingError;

use self::{
    coinflip::{CoinFlipConfig, CoinFlipInput},
    crash::{CrashConfig, CrashInput},
};

use super::{vrf_result::VrfResult, StateAccountType};

pub mod coinflip;
pub mod crash;

#[derive(BorshDeserialize, BorshSerialize, Clone, ShankAccount)]
pub struct Game {
    pub account_type: StateAccountType,
    pub host: Pubkey,
    pub is_active: bool,
    pub unresolved_vrf_result: u32,
    pub total_lamports_in: u64,
    pub total_lamports_out: u64,
    pub common_config: CommonGameConfig,
    pub game_type_config: GameTypeConfig,
}

impl Game {
    pub fn new(host: Pubkey, min_wager: u64, max_wager: u64, game_type_config: GameTypeConfig) -> Self {
        Self {
            account_type: StateAccountType::Game,
            host,
            is_active: true,
            unresolved_vrf_result: 0,
            total_lamports_in: 0,
            total_lamports_out: 0,
            common_config: CommonGameConfig { min_wager, max_wager },
            game_type_config,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CommonGameConfig {
    pub min_wager: u64,
    pub max_wager: u64,
}
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub enum GameTypeConfig {
    CoinFlip { config: CoinFlipConfig },
    Crash { config: CrashConfig },
}
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub enum BetInput {
    CoinFlip { input: CoinFlipInput },
    Crash { input: CrashInput },
}
pub trait ProcessVrfResult {
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<u64, ProgramError>;
    fn check_vrf_result(vrf_result: &VrfResult) -> Result<(), ProgramError> {
        if !vrf_result.is_fullfilled {
            Err(ProgramError::from(BettingError::VrfResultNotFullfilled))
        } else if vrf_result.is_used {
            Err(ProgramError::from(BettingError::VrfResultAlreadyUsed))
        } else {
            Ok(())
        }
    }
}
pub trait CheckBetInput {
    fn check_bet_input(&self, game: &Game) -> Result<(), ProgramError>;
}

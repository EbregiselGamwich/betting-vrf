use std::convert::TryInto;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::{self, ProgramError};

use crate::state::{user_account::UserAccount, vrf_result::VrfResult};

use super::{BetInput, CheckBetInput, Game, GameTypeConfig, ProcessVrfResult};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CoinFlipConfig {
    pub host_probability_advantage: u64,
    pub payout_rate: u64,
}
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CoinFlipInput {
    pub wager: u64,
    pub side: CoinFlipSide,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoinFlipSide {
    Head,
    Tail,
}

impl CheckBetInput for CoinFlipInput {
    fn check_bet_input(&self, game: &Game) -> Result<(), program_error::ProgramError> {
        if !matches!(game.game_type_config, GameTypeConfig::CoinFlip { .. })
            || self.wager < game.common_config.min_wager
            || self.wager > game.common_config.max_wager
        {
            Err(ProgramError::InvalidArgument)
        } else {
            Ok(())
        }
    }

    fn check_bettor_balance(&self, _game: &Game, user_account: &UserAccount) -> Result<u64, ProgramError> {
        if user_account.current_lamports >= self.wager {
            Ok(self.wager)
        } else {
            Err(ProgramError::InsufficientFunds)
        }
    }

    fn check_host_balance(&self, game: &Game, user_account: &UserAccount) -> Result<u64, ProgramError> {
        if let GameTypeConfig::CoinFlip { config } = game.game_type_config {
            let payout_if_bettor_win = self.wager * config.payout_rate / 10000;
            if user_account.current_lamports >= payout_if_bettor_win {
                Ok(payout_if_bettor_win)
            } else {
                Err(ProgramError::InsufficientFunds)
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }
}
impl ProcessVrfResult for CoinFlipConfig {
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<bool, ProgramError> {
        self.check_vrf_result(vrf_result)?;
        if let BetInput::CoinFlip {
            input: CoinFlipInput { wager: _, side },
        } = vrf_result.bet_input
        {
            let mut rand_bytes: [u8; 16] = Default::default();
            rand_bytes.copy_from_slice(&vrf_result.beta[0..16]);
            let rand_number: u64 = (u128::from_le_bytes(rand_bytes) & 10000).try_into().unwrap();
            match side {
                CoinFlipSide::Head => {
                    if rand_number < 5000 - self.host_probability_advantage {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                CoinFlipSide::Tail => {
                    if rand_number > 5000 + self.host_probability_advantage {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }
}

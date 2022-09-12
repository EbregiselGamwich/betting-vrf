use std::convert::TryInto;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::{self, ProgramError};

use crate::state::vrf_result::VrfResult;

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

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum CoinFlipSide {
    Head,
    Tail,
}

impl CheckBetInput for CoinFlipInput {
    fn check_bet_input(&self, game: &Game) -> Result<(), program_error::ProgramError> {
        if !matches!(game.game_type_config, GameTypeConfig::CoinFlip { .. }) || self.wager < game.min_wager || self.wager > game.max_wager {
            Err(ProgramError::InvalidArgument)
        } else {
            Ok(())
        }
    }
}
impl ProcessVrfResult for CoinFlipConfig {
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<u64, ProgramError> {
        CoinFlipConfig::check_vrf_result(vrf_result)?;
        if let BetInput::CoinFlip {
            input: CoinFlipInput { wager, side },
        } = vrf_result.bet_input
        {
            let mut rand_bytes: [u8; 16] = Default::default();
            rand_bytes.copy_from_slice(&vrf_result.beta[0..16]);
            let rand_number: u64 = (u128::from_le_bytes(rand_bytes) & 10000).try_into().unwrap();
            match side {
                CoinFlipSide::Head => {
                    if rand_number < 5000 - self.host_probability_advantage {
                        Ok(wager + wager * (self.payout_rate) / 10000)
                    } else {
                        Ok(0)
                    }
                }
                CoinFlipSide::Tail => {
                    if rand_number > 5000 + self.host_probability_advantage {
                        Ok(wager + wager * (self.payout_rate) / 10000)
                    } else {
                        Ok(0)
                    }
                }
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }
}
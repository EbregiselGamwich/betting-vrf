use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

use crate::state::{user_account::UserAccount, vrf_result::VrfResult};

use super::{BetInput, CheckBetInput, Game, GameTypeConfig, ProcessVrfResult};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CrashConfig {
    pub multiplier_straight_one_possibility: u64,
}
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct CrashInput {
    pub target_multiplier: u64,
    pub wager: u64,
}

impl CheckBetInput for CrashInput {
    fn check_bet_input(&self, game: &Game) -> Result<(), ProgramError> {
        if !matches!(game.game_type_config, GameTypeConfig::Crash { .. })
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

    fn check_host_balance(&self, _game: &Game, user_account: &UserAccount) -> Result<u64, ProgramError> {
        let target_f = self.target_multiplier as f64 / 100.0;
        let payout_if_bettor_win = (self.wager as f64 * target_f).floor() as u64;
        if user_account.current_lamports >= payout_if_bettor_win {
            Ok(payout_if_bettor_win)
        } else {
            Err(ProgramError::InsufficientFunds)
        }
    }
}
impl ProcessVrfResult for CrashConfig {
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<bool, ProgramError> {
        self.check_vrf_result(vrf_result)?;
        if let BetInput::Crash {
            input: CrashInput { target_multiplier, wager: _ },
        } = vrf_result.bet_input
        {
            let target_f = target_multiplier as f64 / 100.0;
            // get random number
            let mut rand_bytes: [u8; 16] = Default::default();
            rand_bytes.copy_from_slice(&vrf_result.beta[0..16]);
            let rand_number: u128 = u128::from_le_bytes(rand_bytes);
            // calculate multiplier
            let divisor = (1.0 / (self.multiplier_straight_one_possibility as f64 / 10000.0)) as u128;
            let multiplier = if rand_number % divisor == 0 {
                1.0
            } else {
                ((100.0 * u64::MAX as f64 - rand_number as f64) / (u64::MAX as f64 - rand_number as f64)) / 100.0
            };
            // result
            if target_f <= multiplier {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }
}

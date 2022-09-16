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
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<(u64, u64), ProgramError> {
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
                Ok((vrf_result.locked_bettor_lamports, vrf_result.locked_host_lamports))
            } else {
                Ok((vrf_result.locked_host_lamports + vrf_result.locked_bettor_lamports, 0))
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod test {
    use solana_program::{program_error::ProgramError, pubkey::Pubkey};

    use crate::state::{
        game::{
            coinflip::{CoinFlipConfig, CoinFlipInput, CoinFlipSide::Head},
            BetInput, CheckBetInput, Game, GameTypeConfig, ProcessVrfResult,
        },
        user_account::UserAccount,
        vrf_result::VrfResult,
    };

    use super::{CrashConfig, CrashInput};

    #[test]
    fn test_crash_check_bet_input() {
        let input = CrashInput {
            target_multiplier: 120,
            wager: 1000,
        };
        let game = Game::new(
            Pubkey::new_unique(),
            1000,
            10000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        // ok
        assert!(input.check_bet_input(&game).is_ok());
        // wrong game type
        let game = Game::new(
            Pubkey::new_unique(),
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
                },
            },
        );
        assert!(matches!(input.check_bet_input(&game).unwrap_err(), ProgramError::InvalidArgument));
        // wager too low
        let game = Game::new(
            Pubkey::new_unique(),
            10000,
            100000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        assert!(matches!(input.check_bet_input(&game).unwrap_err(), ProgramError::InvalidArgument));
        // wager too high
        let game = Game::new(
            Pubkey::new_unique(),
            10,
            100,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        assert!(matches!(input.check_bet_input(&game).unwrap_err(), ProgramError::InvalidArgument));
    }
    #[test]
    fn test_crash_check_bettor_balance() {
        let input = CrashInput {
            target_multiplier: 120,
            wager: 1000,
        };
        let game = Game::new(
            Pubkey::new_unique(),
            1000,
            10000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        let mut user_account = UserAccount::new(Pubkey::new_unique(), None, None);
        user_account.current_lamports = 5000;

        // ok
        assert_eq!(input.check_bettor_balance(&game, &user_account).unwrap(), 1000);
        // not enough money
        user_account.current_lamports = 1;
        assert!(matches!(
            input.check_bettor_balance(&game, &user_account).unwrap_err(),
            ProgramError::InsufficientFunds
        ));
    }
    #[test]
    fn test_crash_check_host_balance() {
        let input = CrashInput {
            target_multiplier: 120,
            wager: 1000,
        };
        let game = Game::new(
            Pubkey::new_unique(),
            1000,
            10000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        let mut user_account = UserAccount::new(Pubkey::new_unique(), None, None);
        user_account.current_lamports = 5000;

        // ok
        assert_eq!(input.check_host_balance(&game, &user_account).unwrap(), 1200);
        // not enough money
        user_account.current_lamports = 1;
        assert!(matches!(
            input.check_host_balance(&game, &user_account).unwrap_err(),
            ProgramError::InsufficientFunds
        ));
    }

    #[test]
    fn test_crash_process_vrf() {
        let game_config = CrashConfig {
            multiplier_straight_one_possibility: 100,
        };
        let mut vrf_result = VrfResult::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            [0; 72],
            1000,
            1200,
            BetInput::Crash {
                input: CrashInput {
                    target_multiplier: 120,
                    wager: 1000,
                },
            },
        );
        vrf_result.is_fullfilled = true;

        // target multiplier hit
        vrf_result.beta[0..16].copy_from_slice(&(u64::MAX as u128 / 2).to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 1000);
        assert_eq!(bettor_gain, 1200);
        // target multiplier miss
        vrf_result.beta[0..16].copy_from_slice(&(u64::MAX as u128 / 100).to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 2200);
        assert_eq!(bettor_gain, 0);
        // wrong input type
        let mut vrf_result = VrfResult::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            [0; 72],
            1000,
            1200,
            BetInput::CoinFlip {
                input: CoinFlipInput { wager: 1000, side: Head },
            },
        );
        vrf_result.is_fullfilled = true;
        assert!(matches!(
            game_config.process_vrf_result(&vrf_result).unwrap_err(),
            ProgramError::InvalidArgument
        ));
    }
}

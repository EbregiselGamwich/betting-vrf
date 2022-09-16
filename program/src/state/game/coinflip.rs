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
    fn process_vrf_result(&self, vrf_result: &VrfResult) -> Result<(u64, u64), ProgramError> {
        self.check_vrf_result(vrf_result)?;
        if let BetInput::CoinFlip {
            input: CoinFlipInput { wager: _, side },
        } = vrf_result.bet_input
        {
            let mut rand_bytes: [u8; 16] = Default::default();
            rand_bytes.copy_from_slice(&vrf_result.beta[0..16]);
            let rand_number: u64 = (u128::from_le_bytes(rand_bytes) % 10000).try_into().unwrap();
            match side {
                CoinFlipSide::Head => {
                    if rand_number < 5000 - self.host_probability_advantage {
                        Ok((0, vrf_result.locked_bettor_lamports + vrf_result.locked_host_lamports))
                    } else {
                        Ok((vrf_result.locked_bettor_lamports + vrf_result.locked_host_lamports, 0))
                    }
                }
                CoinFlipSide::Tail => {
                    if rand_number > 5000 + self.host_probability_advantage {
                        Ok((0, vrf_result.locked_bettor_lamports + vrf_result.locked_host_lamports))
                    } else {
                        Ok((vrf_result.locked_bettor_lamports + vrf_result.locked_host_lamports, 0))
                    }
                }
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
            crash::{CrashConfig, CrashInput},
            BetInput, CheckBetInput, Game, GameTypeConfig, ProcessVrfResult,
        },
        user_account::UserAccount,
        vrf_result::VrfResult,
    };

    use super::{CoinFlipConfig, CoinFlipInput, CoinFlipSide};

    #[test]
    fn test_coinflip_check_bet_input() {
        let input = CoinFlipInput {
            wager: 2000,
            side: CoinFlipSide::Head,
        };
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
        assert!(input.check_bet_input(&game).is_ok());

        // wrong game type
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
        assert!(matches!(input.check_bet_input(&game).unwrap_err(), ProgramError::InvalidArgument));

        // less than min wager
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

        // more than max wager
        let game = Game::new(
            Pubkey::new_unique(),
            100,
            1000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
                },
            },
        );
        assert!(matches!(input.check_bet_input(&game).unwrap_err(), ProgramError::InvalidArgument));
    }
    #[test]
    fn test_coinflip_check_bettor_balance() {
        let input = CoinFlipInput {
            wager: 2000,
            side: CoinFlipSide::Head,
        };
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
        let mut user_account = UserAccount::new(Pubkey::new_unique(), None, None);
        user_account.current_lamports = 5000;

        // ok
        assert_eq!(input.check_bettor_balance(&game, &user_account).unwrap(), 2000);

        // not enough money
        user_account.current_lamports = 1;
        assert!(matches!(
            input.check_bettor_balance(&game, &user_account).unwrap_err(),
            ProgramError::InsufficientFunds
        ));
    }
    #[test]
    fn test_coinflip_check_host_balance() {
        let input = CoinFlipInput {
            wager: 2000,
            side: CoinFlipSide::Head,
        };
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
        let mut user_account = UserAccount::new(Pubkey::new_unique(), None, None);
        user_account.current_lamports = 5000;

        // ok
        assert_eq!(input.check_host_balance(&game, &user_account).unwrap(), 2000 * 9900 / 10000);
        // not enough money
        user_account.current_lamports = 1;
        assert!(matches!(
            input.check_host_balance(&game, &user_account).unwrap_err(),
            ProgramError::InsufficientFunds
        ));
        // wrong game type
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
        assert!(matches!(
            input.check_host_balance(&game, &user_account).unwrap_err(),
            ProgramError::InvalidArgument
        ));
    }
    #[test]
    fn test_coinflip_process_vrf() {
        let game_config = CoinFlipConfig {
            host_probability_advantage: 100,
            payout_rate: 9900,
        };
        let mut vrf_result = VrfResult::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            [0; 72],
            2000,
            2000,
            BetInput::CoinFlip {
                input: CoinFlipInput {
                    wager: 2000,
                    side: CoinFlipSide::Head,
                },
            },
        );
        vrf_result.is_fullfilled = true;
        // bettor win head
        vrf_result.beta[0..16].copy_from_slice(&2000_u128.to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 0);
        assert_eq!(bettor_gain, 4000);
        // bettor lose head with host probability advantage
        vrf_result.beta[0..16].copy_from_slice(&4901_u128.to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 4000);
        assert_eq!(bettor_gain, 0);
        // bettor lose tail with host probability advantage
        vrf_result.bet_input = BetInput::CoinFlip {
            input: CoinFlipInput {
                wager: 2000,
                side: CoinFlipSide::Tail,
            },
        };
        vrf_result.beta[0..16].copy_from_slice(&5100_u128.to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 4000);
        assert_eq!(bettor_gain, 0);
        // bettor win tail
        vrf_result.beta[0..16].copy_from_slice(&8000_u128.to_le_bytes());
        let (host_gain, bettor_gain) = game_config.process_vrf_result(&vrf_result).unwrap();
        assert_eq!(host_gain, 0);
        assert_eq!(bettor_gain, 4000);

        // wrong input type
        vrf_result.bet_input = BetInput::Crash {
            input: CrashInput {
                target_multiplier: 120,
                wager: 2000,
            },
        };
        assert!(matches!(
            game_config.process_vrf_result(&vrf_result).unwrap_err(),
            ProgramError::InvalidArgument
        ));
    }
}

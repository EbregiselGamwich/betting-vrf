use std::convert::TryInto;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    state::{
        game::{CommonGameConfig, Game, GameTypeConfig},
        stats::Stats,
        user_account::UserAccount,
        BettingAccount,
    },
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct GameCreateArgs {
    pub common_config: CommonGameConfig,
    pub game_type_config: GameTypeConfig,
}

pub fn game_create(program_id: &Pubkey, accounts: &[AccountInfo], args: GameCreateArgs) -> ProgramResult {
    msg!("Instruction: GameCreate");
    // get accounts
    let iter = &mut accounts.iter();

    let host_account_info = next_account_info(iter)?;
    let host_user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let game_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut host_user_account_state = UserAccount::try_from_account_info(host_user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(host_account_info)?;
    check_is_writable(host_account_info)?;

    check_is_writable(host_user_account_info)?;
    check_pda_cannonical_bump(host_user_account_info, &[b"UserAccount".as_ref(), host_user_account_state.authority.as_ref()])?;
    check_pubkey_eq(host_account_info, &host_user_account_state.authority)?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_is_writable(game_account_info)?;
    let common_config_vec = args.common_config.try_to_vec()?;
    let game_type_config_vec = args.game_type_config.try_to_vec()?;
    let game_pda_bump = check_pda_cannonical_bump(
        game_account_info,
        &[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()],
    )?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;
    // update host user account
    host_user_account_state.games_hosted += 1;
    host_user_account_state.serialize(&mut &mut host_user_account_info.data.borrow_mut()[..])?;
    // update stats account
    stats_account_state.total_games += 1;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;
    // create game account
    let game_pda_signer_seeds = &[
        b"Game".as_ref(),
        common_config_vec.as_slice(),
        game_type_config_vec.as_slice(),
        &[game_pda_bump],
    ];
    let game_state = Game::new(
        *host_account_info.key,
        args.common_config.min_wager,
        args.common_config.max_wager,
        args.game_type_config,
    );
    let game_data = game_state.try_to_vec()?;
    let game_data_len = game_data.len();
    let min_rent = Rent::get()?.minimum_balance(game_data_len);
    let game_create_ix = system_instruction::create_account(
        host_account_info.key,
        game_account_info.key,
        min_rent,
        game_data_len.try_into().unwrap(),
        program_id,
    );
    invoke_signed(
        &game_create_ix,
        &[host_account_info.clone(), game_account_info.clone()],
        &[game_pda_signer_seeds],
    )?;
    game_account_info.data.borrow_mut().copy_from_slice(&game_data);

    Ok(())
}

#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

    use crate::{
        instructions::BettingInstruction,
        state::{
            game::{coinflip::CoinFlipConfig, CommonGameConfig, Game, GameTypeConfig},
            stats::Stats,
            user_account::UserAccount,
            StateAccountType,
        },
    };

    use super::GameCreateArgs;

    #[tokio::test]
    async fn test_game_create_success() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let user = Keypair::new();
        program_test.add_account(
            user.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let referral = Pubkey::new_unique();
        let (user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), user.pubkey().as_ref()], &program_id);
        let user_account_state = UserAccount::new(user.pubkey(), Some(referral), Some("Username".to_string()));
        let user_account_data = user_account_state.try_to_vec().unwrap();
        let user_account_data_len = user_account_data.len();
        program_test.add_account(
            user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(user_account_data_len),
                data: user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        let stats_data = stats_state.try_to_vec().unwrap();
        let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: Rent::default().minimum_balance(stats_data_len),
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let game_common_config = CommonGameConfig {
            min_wager: 1000,
            max_wager: 10000,
        };
        let game_type_config = GameTypeConfig::CoinFlip {
            config: CoinFlipConfig {
                host_probability_advantage: 100,
                payout_rate: 9900,
            },
        };
        let common_config_vec = game_common_config.try_to_vec().unwrap();
        let game_type_config_vec = game_type_config.try_to_vec().unwrap();
        let (game_pda, _) = Pubkey::find_program_address(&[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()], &program_id);

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GameCreate {
                    args: GameCreateArgs {
                        common_config: game_common_config,
                        game_type_config,
                    },
                },
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.games_hosted, 1);
        // the stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_games, 1);
        // the game pda account should be created
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.account_type, StateAccountType::Game);
        assert_eq!(game_state.host, user.pubkey());
        assert!(game_state.is_active);
        assert_eq!(game_state.unresolved_vrf_result, 0);
        assert_eq!(game_state.total_lamports_in, 0);
        assert_eq!(game_state.total_lamports_out, 0);
        assert_eq!(game_state.common_config.max_wager, game_common_config.max_wager);
        assert_eq!(game_state.common_config.min_wager, game_common_config.min_wager);
        match game_state.game_type_config {
            GameTypeConfig::CoinFlip { config } => {
                assert_eq!(config.host_probability_advantage, 100);
                assert_eq!(config.payout_rate, 9900);
            }
            _ => panic!(),
        }
    }
}

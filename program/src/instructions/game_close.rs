use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    error::BettingError,
    state::{game::Game, stats::Stats, user_account::UserAccount, BettingAccount},
};

pub fn game_close(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: GameClose");
    // get accounts
    let iter = &mut accounts.iter();

    let host_wallet_account_info = next_account_info(iter)?;
    let host_user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let game_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut host_user_account_state = UserAccount::try_from_account_info(host_user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    let game_account_state = Game::try_from_account_info(game_account_info)?;
    // check accounts
    check_is_signer(host_wallet_account_info)?;
    check_is_writable(host_wallet_account_info)?;
    check_pubkey_eq(host_wallet_account_info, &host_user_account_state.authority)?;

    check_is_writable(host_user_account_info)?;
    check_pda_cannonical_bump(host_user_account_info, &[b"UserAccount".as_ref(), host_user_account_state.authority.as_ref()])?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_is_writable(game_account_info)?;
    let game_common_config_vec = game_account_state.common_config.try_to_vec()?;
    let game_type_config_vec = game_account_state.game_type_config.try_to_vec()?;
    check_pda_cannonical_bump(
        game_account_info,
        &[b"Game".as_ref(), game_common_config_vec.as_slice(), game_type_config_vec.as_slice()],
    )?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;
    // check authority
    if &game_account_state.host != host_wallet_account_info.key {
        msg!(
            "Expect account {} to have authority over game account {}",
            host_wallet_account_info.key,
            game_account_info.key
        );
        return Err(ProgramError::from(BettingError::NoAuthority));
    }
    // check game close conditions
    if game_account_state.unresolved_vrf_result > 0 {
        msg!("Game {} is not settled");
        return Err(ProgramError::from(BettingError::GameNotSettled));
    }

    // update host user account
    host_user_account_state.games_hosted -= 1;
    host_user_account_state.serialize(&mut &mut host_user_account_info.data.borrow_mut()[..])?;
    // update stats account
    stats_account_state.total_games -= 1;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;
    // close game account
    game_account_info.data.borrow_mut().fill(0);
    game_account_info.realloc(0, false)?;
    **host_wallet_account_info.lamports.borrow_mut() = host_wallet_account_info.lamports().checked_add(game_account_info.lamports()).unwrap();
    **game_account_info.lamports.borrow_mut() = 0;

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
            game::{coinflip::CoinFlipConfig, Game, GameTypeConfig},
            stats::Stats,
            user_account::UserAccount,
        },
    };

    #[tokio::test]
    async fn test_game_close_success() {
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
        let mut user_account_state = UserAccount::new(user.pubkey(), Some(referral), Some("Username".to_string()));
        user_account_state.games_hosted = 1;
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
        stats_state.total_games = 1;
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

        let game_state = Game::new(
            user.pubkey(),
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
                },
            },
        );
        let common_config_vec = game_state.common_config.try_to_vec().unwrap();
        let game_type_config_vec = game_state.game_type_config.try_to_vec().unwrap();
        let game_data = game_state.try_to_vec().unwrap();
        let game_data_len = game_data.len();
        let (game_pda, _) = Pubkey::find_program_address(&[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()], &program_id);
        program_test.add_account(
            game_pda,
            Account {
                lamports: Rent::default().minimum_balance(game_data_len),
                data: game_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GameClose,
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

        // the rent should be returned to the user wallet account
        let uesr_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(uesr_wallet_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(game_data_len));
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.games_hosted, 0);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_games, 0);
        // the game account should be closed
        assert!(banks_client.get_account(game_pda).await.unwrap().is_none());
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(0)")]
    async fn test_game_close_err_no_authority() {
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
        let mut user_account_state = UserAccount::new(user.pubkey(), Some(referral), Some("Username".to_string()));
        user_account_state.games_hosted = 1;
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
        stats_state.total_games = 1;
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

        let game_state = Game::new(
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
        let common_config_vec = game_state.common_config.try_to_vec().unwrap();
        let game_type_config_vec = game_state.game_type_config.try_to_vec().unwrap();
        let game_data = game_state.try_to_vec().unwrap();
        let game_data_len = game_data.len();
        let (game_pda, _) = Pubkey::find_program_address(&[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()], &program_id);
        program_test.add_account(
            game_pda,
            Account {
                lamports: Rent::default().minimum_balance(game_data_len),
                data: game_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GameClose,
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

        // the rent should be returned to the user wallet account
        let uesr_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(uesr_wallet_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(game_data_len));
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.games_hosted, 0);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_games, 0);
        // the game account should be closed
        assert!(banks_client.get_account(game_pda).await.unwrap().is_none());
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(13)")]
    async fn test_game_close_err_unresolved_vrf_results_remaining() {
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
        let mut user_account_state = UserAccount::new(user.pubkey(), Some(referral), Some("Username".to_string()));
        user_account_state.games_hosted = 1;
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
        stats_state.total_games = 1;
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

        let mut game_state = Game::new(
            user.pubkey(),
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
                },
            },
        );
        game_state.unresolved_vrf_result = 1;
        let common_config_vec = game_state.common_config.try_to_vec().unwrap();
        let game_type_config_vec = game_state.game_type_config.try_to_vec().unwrap();
        let game_data = game_state.try_to_vec().unwrap();
        let game_data_len = game_data.len();
        let (game_pda, _) = Pubkey::find_program_address(&[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()], &program_id);
        program_test.add_account(
            game_pda,
            Account {
                lamports: Rent::default().minimum_balance(game_data_len),
                data: game_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GameClose,
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

        // the rent should be returned to the user wallet account
        let uesr_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(uesr_wallet_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(game_data_len));
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.games_hosted, 0);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_games, 0);
        // the game account should be closed
        assert!(banks_client.get_account(game_pda).await.unwrap().is_none());
    }
}

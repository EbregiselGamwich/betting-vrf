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
    state::{stats::Stats, user_account::UserAccount, BettingAccount},
};

pub fn user_account_close(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: UserAccountClose");
    // get accounts
    let iter = &mut accounts.iter();

    let user_wallet_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let user_account_state = UserAccount::try_from_account_info(user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(user_wallet_account_info)?;
    check_is_writable(user_wallet_account_info)?;

    check_is_writable(user_account_info)?;
    check_pda_cannonical_bump(user_account_info, &[b"UserAccount".as_ref(), user_account_state.authority.as_ref()])?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;
    // check authority
    if &user_account_state.authority != user_wallet_account_info.key {
        msg!(
            "Expect account {} to have authority over account {}",
            user_wallet_account_info.key,
            user_account_info.key
        );
        return Err(ProgramError::from(BettingError::NoAuthority));
    }
    // check lamports
    if user_account_state.current_lamports > 0 {
        msg!("Lamports must be withdrew before account {} can be closed", user_account_info.key);
        return Err(ProgramError::from(BettingError::UserAccountNotSettled));
    }
    // check if the user is still hosting games
    if user_account_state.games_hosted > 0 {
        msg!("Account {} is still hosting games", user_account_info.key);
        return Err(ProgramError::from(BettingError::UserAccountNotSettled));
    }
    // check if the user has unresolved bets left
    if user_account_state.active_vrf_results > 0 {
        msg!("Account {} still has active VRF results left", user_account_info.key);
        return Err(ProgramError::from(BettingError::UserAccountNotSettled));
    }
    // update user wallet account
    **user_wallet_account_info.lamports.borrow_mut() = user_wallet_account_info.lamports().checked_add(user_account_info.lamports()).unwrap();
    // close user account
    user_account_info.data.borrow_mut().fill(0);
    user_account_info.realloc(0, false)?;
    **user_account_info.lamports.borrow_mut() = 0;
    // update stats account
    stats_account_state.total_users -= 1;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;

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
        state::{stats::Stats, user_account::UserAccount},
    };

    #[tokio::test]
    async fn test_user_account_close_success() {
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

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountClose,
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account should be closed
        assert!(banks_client.get_account(user_account_pda).await.unwrap().is_none());
        // the rent should be returned to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(
            user_wallet_account.lamports,
            LAMPORTS_PER_SOL + Rent::default().minimum_balance(user_account_data_len)
        );
        // the stats account should be updated
        let stats_account_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_account_state.total_users, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(0)")]
    async fn test_user_account_close_err_wrong_authority() {
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
        let wrong_authority_user = Keypair::new();
        program_test.add_account(
            wrong_authority_user.pubkey(),
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

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountClose,
                vec![
                    AccountMeta::new(wrong_authority_user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&wrong_authority_user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account should be closed
        assert!(banks_client.get_account(user_account_pda).await.unwrap().is_none());
        // the rent should be returned to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(
            user_wallet_account.lamports,
            LAMPORTS_PER_SOL + Rent::default().minimum_balance(user_account_data_len)
        );
        // the stats account should be updated
        let stats_account_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_account_state.total_users, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(8)")]
    async fn test_user_account_close_err_user_account_not_settled() {
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
        user_account_state.current_lamports = 10000;
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

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountClose,
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account should be closed
        assert!(banks_client.get_account(user_account_pda).await.unwrap().is_none());
        // the rent should be returned to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(
            user_wallet_account.lamports,
            LAMPORTS_PER_SOL + Rent::default().minimum_balance(user_account_data_len)
        );
        // the stats account should be updated
        let stats_account_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_account_state.total_users, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(8)")]
    async fn test_user_account_close_err_hosting_games() {
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

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountClose,
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account should be closed
        assert!(banks_client.get_account(user_account_pda).await.unwrap().is_none());
        // the rent should be returned to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(
            user_wallet_account.lamports,
            LAMPORTS_PER_SOL + Rent::default().minimum_balance(user_account_data_len)
        );
        // the stats account should be updated
        let stats_account_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_account_state.total_users, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(8)")]
    async fn test_user_account_close_err_vrf_results_not_closed() {
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
        user_account_state.active_vrf_results = 1;
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

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountClose,
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the user account should be closed
        assert!(banks_client.get_account(user_account_pda).await.unwrap().is_none());
        // the rent should be returned to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(
            user_wallet_account.lamports,
            LAMPORTS_PER_SOL + Rent::default().minimum_balance(user_account_data_len)
        );
        // the stats account should be updated
        let stats_account_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_account_state.total_users, 0);
    }
}

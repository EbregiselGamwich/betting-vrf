use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    constants::OPERATOR_PUBKEY,
    error::BettingError,
    state::{stats::Stats, user_account::UserAccount, BettingAccount},
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct UserAccountWithdrawArgs {
    pub lamports: u64,
}
pub fn user_account_withdraw(_program_id: &Pubkey, accounts: &[AccountInfo], args: UserAccountWithdrawArgs) -> ProgramResult {
    msg!("Instruction: UserAccountWithdraw");
    // get accounts
    let iter = &mut accounts.iter();

    let user_wallet_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let op_account_info = next_account_info(iter)?;

    let mut user_account_state = UserAccount::try_from_account_info(user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(user_wallet_account_info)?;
    check_is_writable(user_wallet_account_info)?;

    check_is_writable(user_account_info)?;
    check_pda_cannonical_bump(user_account_info, &[b"UserAccount".as_ref(), user_account_state.authority.as_ref()])?;

    check_pubkey_eq(op_account_info, &OPERATOR_PUBKEY)?;
    // check authority
    if user_wallet_account_info.key != &user_account_state.authority {
        msg!(
            "Expect account {} to have authority over account {}",
            user_wallet_account_info.key,
            user_account_info.key
        );
        return Err(ProgramError::from(BettingError::NoAuthority));
    }
    // check withdraw amount
    if user_account_state.current_lamports < args.lamports {
        msg!("Account {} does not have enough lamports", user_account_info.key);
        return Err(ProgramError::InsufficientFunds);
    }
    // calculate transfer amounts
    let profit_share = user_account_state.get_profit_share(args.lamports);
    let user_amount = args.lamports - profit_share;
    let referral_amount = profit_share / 2;
    let op_amount = profit_share - referral_amount;
    // transfer lamports to user wallet account
    **user_wallet_account_info.lamports.borrow_mut() = user_wallet_account_info.lamports().checked_add(user_amount).unwrap();
    // update user account state
    user_account_state.lamports_withdrew += args.lamports;
    user_account_state.current_lamports -= args.lamports;
    user_account_state.serialize(&mut &mut user_account_info.data.borrow_mut()[..])?;
    // update stats account
    stats_account_state.total_lamports_withdrew += args.lamports;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;
    **stats_account_info.lamports.borrow_mut() = stats_account_info.lamports().checked_sub(args.lamports).unwrap();

    // transfer lamports to op
    **op_account_info.lamports.borrow_mut() = op_account_info.lamports().checked_add(op_amount).unwrap();

    // if there's a referral for the user
    if let Some(referral) = user_account_state.referral {
        // get referral account
        let referral_account_info = next_account_info(iter)?;
        // check referral account
        check_is_writable(referral_account_info)?;
        check_pubkey_eq(referral_account_info, &referral)?;
        // transfer lamports
        **referral_account_info.lamports.borrow_mut() = referral_account_info.lamports().checked_add(referral_amount).unwrap();
    } else {
        // no referral, profit share goes to operator
        **op_account_info.lamports.borrow_mut() = op_account_info.lamports().checked_add(referral_amount).unwrap();
    }

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
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

    use crate::{
        constants::OPERATOR_PUBKEY,
        instructions::BettingInstruction,
        state::{stats::Stats, user_account::UserAccount},
    };

    use super::UserAccountWithdrawArgs;

    #[tokio::test]
    async fn test_user_account_withdraw_success_with_referral() {
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
        user_account_state.current_lamports = 20000;
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
        // let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        program_test.add_account(
            OPERATOR_PUBKEY,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        program_test.add_account(
            referral,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountWithdraw {
                    args: UserAccountWithdrawArgs { lamports: 10000 },
                },
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(OPERATOR_PUBKEY, false),
                    AccountMeta::new(referral, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // lamports should be transferred to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL + 9900);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_withdrew, 10000);
        // lamports should be transferred from the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, LAMPORTS_PER_SOL - 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_withdrew, 10000);
        // profit share should be transferred to the operator account
        let op_account = banks_client.get_account(OPERATOR_PUBKEY).await.unwrap().unwrap();
        assert_eq!(op_account.lamports, LAMPORTS_PER_SOL + 50);
        // profit share should be transferred to the referral wallet account
        let referral_account = banks_client.get_account(referral).await.unwrap().unwrap();
        assert_eq!(referral_account.lamports, LAMPORTS_PER_SOL + 50);
    }

    #[tokio::test]
    async fn test_user_account_withdraw_success_without_referral() {
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

        let (user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), user.pubkey().as_ref()], &program_id);
        let mut user_account_state = UserAccount::new(user.pubkey(), None, Some("Username".to_string()));
        user_account_state.current_lamports = 20000;
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
        // let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        program_test.add_account(
            OPERATOR_PUBKEY,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountWithdraw {
                    args: UserAccountWithdrawArgs { lamports: 10000 },
                },
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(OPERATOR_PUBKEY, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // lamports should be transferred to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL + 9900);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_withdrew, 10000);
        // lamports should be transferred from the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, LAMPORTS_PER_SOL - 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_withdrew, 10000);
        // profit share should be transferred to the operator account
        let op_account = banks_client.get_account(OPERATOR_PUBKEY).await.unwrap().unwrap();
        assert_eq!(op_account.lamports, LAMPORTS_PER_SOL + 100);
    }

    #[tokio::test]
    async fn test_user_account_withdraw_success_user_no_profit() {
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
        user_account_state.current_lamports = 20000;
        user_account_state.lamports_deposited = 20000;
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
        // let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        program_test.add_account(
            OPERATOR_PUBKEY,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        program_test.add_account(
            referral,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountWithdraw {
                    args: UserAccountWithdrawArgs { lamports: 10000 },
                },
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(OPERATOR_PUBKEY, false),
                    AccountMeta::new(referral, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // lamports should be transferred to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL + 10000);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_withdrew, 10000);
        // lamports should be transferred from the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, LAMPORTS_PER_SOL - 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_withdrew, 10000);
        // no profit share should be transferred to the operator account
        let op_account = banks_client.get_account(OPERATOR_PUBKEY).await.unwrap().unwrap();
        assert_eq!(op_account.lamports, LAMPORTS_PER_SOL);
        // no profit share should be transferred to the referral wallet account
        let referral_account = banks_client.get_account(referral).await.unwrap().unwrap();
        assert_eq!(referral_account.lamports, LAMPORTS_PER_SOL);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(0)")]
    async fn test_user_account_withdraw_err_wrong_authority() {
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
        let mut user_account_state = UserAccount::new(user.pubkey(), Some(referral), Some("Username".to_string()));
        user_account_state.current_lamports = 20000;
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
        // let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        program_test.add_account(
            OPERATOR_PUBKEY,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        program_test.add_account(
            referral,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountWithdraw {
                    args: UserAccountWithdrawArgs { lamports: 10000 },
                },
                vec![
                    AccountMeta::new(wrong_authority_user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(OPERATOR_PUBKEY, false),
                    AccountMeta::new(referral, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&wrong_authority_user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // lamports should be transferred to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL + 9900);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_withdrew, 10000);
        // lamports should be transferred from the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, LAMPORTS_PER_SOL - 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_withdrew, 10000);
        // profit share should be transferred to the operator account
        let op_account = banks_client.get_account(OPERATOR_PUBKEY).await.unwrap().unwrap();
        assert_eq!(op_account.lamports, LAMPORTS_PER_SOL + 50);
        // profit share should be transferred to the referral wallet account
        let referral_account = banks_client.get_account(referral).await.unwrap().unwrap();
        assert_eq!(referral_account.lamports, LAMPORTS_PER_SOL + 50);
    }

    #[tokio::test]
    #[should_panic(expected = "InsufficientFunds")]
    async fn test_user_account_withdraw_err_insufficient_funds() {
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
        user_account_state.current_lamports = 20000;
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
        // let stats_data_len = stats_data.len();
        program_test.add_account(
            stats_pda,
            Account {
                lamports: LAMPORTS_PER_SOL,
                data: stats_data,
                owner: program_id,
                ..Default::default()
            },
        );

        program_test.add_account(
            OPERATOR_PUBKEY,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        program_test.add_account(
            referral,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::UserAccountWithdraw {
                    args: UserAccountWithdrawArgs { lamports: 100000000 },
                },
                vec![
                    AccountMeta::new(user.pubkey(), true),
                    AccountMeta::new(user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(OPERATOR_PUBKEY, false),
                    AccountMeta::new(referral, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // lamports should be transferred to the user wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL + 9900);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_withdrew, 10000);
        // lamports should be transferred from the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, LAMPORTS_PER_SOL - 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_withdrew, 10000);
        // profit share should be transferred to the operator account
        let op_account = banks_client.get_account(OPERATOR_PUBKEY).await.unwrap().unwrap();
        assert_eq!(op_account.lamports, LAMPORTS_PER_SOL + 50);
        // profit share should be transferred to the referral wallet account
        let referral_account = banks_client.get_account(referral).await.unwrap().unwrap();
        assert_eq!(referral_account.lamports, LAMPORTS_PER_SOL + 50);
    }
}

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
    system_instruction, system_program,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    state::{stats::Stats, user_account::UserAccount, BettingAccount},
};
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserAccountDepositArgs {
    pub lamports: u64,
}
pub fn user_account_deposit(_program_id: &Pubkey, accounts: &[AccountInfo], args: UserAccountDepositArgs) -> ProgramResult {
    msg!("Instruction: UserAccountDeposit");
    // get accounts
    let iter = &mut accounts.iter();

    let depositor_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut user_account_state = UserAccount::try_from_account_info(user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(depositor_account_info)?;
    check_is_writable(depositor_account_info)?;

    check_is_writable(user_account_info)?;
    check_pda_cannonical_bump(user_account_info, &[b"UserAccount".as_ref(), user_account_state.authority.as_ref()])?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;

    // transfer lamports
    let lamports_transfer_ix = system_instruction::transfer(depositor_account_info.key, stats_account_info.key, args.lamports);
    invoke(&lamports_transfer_ix, &[depositor_account_info.clone(), stats_account_info.clone()])?;

    // update user account
    user_account_state.current_lamports += args.lamports;
    user_account_state.lamports_deposited += args.lamports;
    user_account_state.serialize(&mut &mut user_account_info.data.borrow_mut()[..])?;

    // update stats account
    stats_account_state.total_lamports_deposited += args.lamports;
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

    use super::UserAccountDepositArgs;

    #[tokio::test]
    async fn test_user_account_deposit_success() {
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
                &BettingInstruction::UserAccountDeposit {
                    args: UserAccountDepositArgs { lamports: 10000 },
                },
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

        // lamports should be transferred from the depositor's wallet account
        let user_wallet_account = banks_client.get_account(user.pubkey()).await.unwrap().unwrap();
        assert_eq!(user_wallet_account.lamports, LAMPORTS_PER_SOL - 10000);
        // user account state should be updated
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.current_lamports, 10000);
        assert_eq!(user_account_state.lamports_deposited, 10000);
        // lamports should be transferred to the stats account
        let stats_account = banks_client.get_account(stats_pda).await.unwrap().unwrap();
        assert_eq!(stats_account.lamports, Rent::default().minimum_balance(stats_data_len) + 10000);
        // stats account state should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_deposited, 10000);
    }
}

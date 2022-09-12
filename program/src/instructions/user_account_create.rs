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
    state::{stats::Stats, user_account::UserAccount, BettingAccount},
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct UserAccountCreateArgs {
    pub username: Option<String>,
    pub referral: Option<Pubkey>,
}

pub fn user_account_create(program_id: &Pubkey, accounts: &[AccountInfo], args: UserAccountCreateArgs) -> ProgramResult {
    msg!("Instruction: UserAccountCreate");
    // get accounts
    let iter = &mut accounts.iter();

    let user_wallet_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(user_wallet_account_info)?;
    check_is_writable(user_wallet_account_info)?;

    check_is_writable(user_account_info)?;
    let user_account_bump = check_pda_cannonical_bump(user_account_info, &[b"UserAccount".as_ref(), user_wallet_account_info.key.as_ref()])?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;
    // create user account
    let user_account_state = UserAccount::new(*user_wallet_account_info.key, args.referral, args.username);
    let user_account_data = user_account_state.try_to_vec()?;
    let user_account_date_len = user_account_data.len();
    let min_rent = Rent::get()?.minimum_balance(user_account_date_len);
    let user_account_signer_seeds = &[b"UserAccount".as_ref(), user_wallet_account_info.key.as_ref(), &[user_account_bump]];
    let user_account_create_ix = system_instruction::create_account(
        user_wallet_account_info.key,
        user_account_info.key,
        min_rent,
        user_account_date_len.try_into().unwrap(),
        program_id,
    );
    invoke_signed(
        &user_account_create_ix,
        &[user_wallet_account_info.clone(), user_account_info.clone()],
        &[user_account_signer_seeds],
    )?;
    user_account_info.data.borrow_mut().copy_from_slice(&user_account_data);
    // update stats account
    stats_account_state.total_users += 1;
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
        state::{stats::Stats, user_account::UserAccount, StateAccountType},
    };

    use super::UserAccountCreateArgs;

    #[tokio::test]
    async fn test_user_account_create_success() {
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let stats_state = Stats::new();
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
                &BettingInstruction::UserAccountCreate {
                    args: UserAccountCreateArgs {
                        username: Some("Username".to_string()),
                        referral: Some(referral),
                    },
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

        // the user account should be created
        let user_account_state: UserAccount = banks_client.get_account_data_with_borsh(user_account_pda).await.unwrap();
        assert_eq!(user_account_state.account_type, StateAccountType::UserAccount);
        assert_eq!(user_account_state.authority, user.pubkey());
        assert_eq!(user_account_state.total_bets, 0);
        assert_eq!(user_account_state.current_lamports, 0);
        assert_eq!(user_account_state.active_vrf_results, 0);
        assert_eq!(user_account_state.referral, Some(referral));
        assert_eq!(user_account_state.username, Some("Username".to_string()));
        assert_eq!(user_account_state.lamports_deposited, 0);
        assert_eq!(user_account_state.lamports_withdrew, 0);
    }
}

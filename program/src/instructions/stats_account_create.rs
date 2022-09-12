use std::convert::TryInto;

use borsh::BorshSerialize;
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
    constants::OPERATOR_PUBKEY,
    state::stats::Stats,
};

pub fn stats_account_create(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: StatsAccountCreate");
    // get accounts
    let iter = &mut accounts.iter();

    let op_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let system_program_accournt_info = next_account_info(iter)?;
    // check accounts
    check_is_signer(op_account_info)?;
    check_is_writable(op_account_info)?;
    check_pubkey_eq(op_account_info, &OPERATOR_PUBKEY)?;

    check_is_writable(stats_account_info)?;
    let stats_account_bump = check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(system_program_accournt_info, &system_program::ID)?;
    // create account
    let stats_account_state = Stats::new();
    let stats_account_data = stats_account_state.try_to_vec()?;
    let stats_account_signer_seeds = &[b"Stats".as_ref(), &[stats_account_bump]];
    let min_rent = Rent::get()?.minimum_balance(stats_account_data.len());
    let stats_account_create_ix = system_instruction::create_account(
        op_account_info.key,
        stats_account_info.key,
        min_rent,
        stats_account_data.len().try_into().unwrap(),
        program_id,
    );
    invoke_signed(
        &stats_account_create_ix,
        &[op_account_info.clone(), stats_account_info.clone()],
        &[stats_account_signer_seeds],
    )?;
    // save state
    stats_account_info.data.borrow_mut().copy_from_slice(&stats_account_data);

    Ok(())
}

#[cfg(test)]
mod test {
    use home::home_dir;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        system_program,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::read_keypair_file, signer::Signer, transaction::Transaction};

    use crate::{
        instructions::BettingInstruction,
        state::{stats::Stats, StateAccountType},
    };

    #[tokio::test]
    async fn test_stats_account_create_success() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        // accounts
        let key_file_path = home_dir().unwrap().join(".config/solana/id.json");
        let operator = read_keypair_file(key_file_path).unwrap();
        program_test.add_account(
            operator.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::StatsAccountCreate,
                vec![
                    AccountMeta::new(operator.pubkey(), true),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&operator, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the stats account should be created
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.account_type, StateAccountType::Stats);
        assert_eq!(stats_state.total_games, 0);
        assert_eq!(stats_state.total_users, 0);
        assert_eq!(stats_state.total_bets, 0);
        assert_eq!(stats_state.total_wager, 0);
        assert_eq!(stats_state.total_lamports_won_by_bettors, 0);
        assert_eq!(stats_state.total_lamports_deposited, 0);
        assert_eq!(stats_state.total_lamports_withdrew, 0);
    }
}

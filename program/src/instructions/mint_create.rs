use std::convert::TryInto;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    constants::OPERATOR_PUBKEY,
    state::{stats::Stats, BettingAccount},
};

pub fn mint_create(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: MintCreate");
    // get accounts
    let iter = &mut accounts.iter();

    let op_account_info = next_account_info(iter)?;
    let mint_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let token_program_account_info = next_account_info(iter)?;
    let system_program_acrcount_info = next_account_info(iter)?;

    let _stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(op_account_info)?;
    check_is_writable(op_account_info)?;
    check_pubkey_eq(op_account_info, &OPERATOR_PUBKEY)?;

    check_is_writable(mint_account_info)?;
    let mint_bump = check_pda_cannonical_bump(mint_account_info, &[b"Mint".as_ref()])?;

    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(token_program_account_info, &spl_token::ID)?;

    check_pubkey_eq(system_program_acrcount_info, &system_program::ID)?;

    // create mint account
    let min_rent = Rent::get()?.minimum_balance(spl_token::state::Mint::LEN);
    let mint_signer_seeds = &[b"Mint".as_ref(), &[mint_bump]];
    let mint_account_create_ix = system_instruction::create_account(
        op_account_info.key,
        mint_account_info.key,
        min_rent,
        spl_token::state::Mint::LEN.try_into().unwrap(),
        &spl_token::ID,
    );
    invoke_signed(
        &mint_account_create_ix,
        &[op_account_info.clone(), mint_account_info.clone()],
        &[mint_signer_seeds],
    )?;
    // init mint
    let mint_init_ix = spl_token::instruction::initialize_mint2(token_program_account_info.key, mint_account_info.key, stats_account_info.key, None, 1)?;
    invoke(&mint_init_ix, &[mint_account_info.clone()])?;

    Ok(())
}

#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use home::home_dir;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        program_option::COption,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::read_keypair_file, signer::Signer, transaction::Transaction};

    use crate::{instructions::BettingInstruction, state::stats::Stats};

    #[tokio::test]
    async fn test_mint_create_success() {
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

        let (mint_pda, _) = Pubkey::find_program_address(&[b"Mint".as_ref()], &program_id);

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
                &BettingInstruction::MintCreate,
                vec![
                    AccountMeta::new(operator.pubkey(), true),
                    AccountMeta::new(mint_pda, false),
                    AccountMeta::new_readonly(stats_pda, false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&operator, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the mint account should be created
        let mint_state: spl_token::state::Mint = banks_client.get_packed_account_data(mint_pda).await.unwrap();
        assert!(mint_state.is_initialized);
        assert_eq!(mint_state.mint_authority, COption::Some(stats_pda));
        assert!(mint_state.freeze_authority.is_none());
        assert_eq!(mint_state.decimals, 1);
        assert_eq!(mint_state.supply, 0);
    }
}

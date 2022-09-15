use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump},
    error::BettingError,
    state::{vrf_result::VrfResult, BettingAccount},
};

pub fn vrf_result_mark_close(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: VrfResultMarkClose");
    // get accounts
    let iter = &mut accounts.iter();

    let bettor_account_info = next_account_info(iter)?;
    let vrf_result_account_info = next_account_info(iter)?;

    let mut vrf_result_state = VrfResult::try_from_account_info(vrf_result_account_info)?;

    // check accounts
    check_is_signer(bettor_account_info)?;

    check_is_writable(vrf_result_account_info)?;
    check_pda_cannonical_bump(
        vrf_result_account_info,
        &[
            b"VrfResult".as_ref(),
            vrf_result_state.game.as_ref(),
            vrf_result_state.owner.as_ref(),
            &vrf_result_state.bet_id.to_le_bytes(),
        ],
    )?;

    // check authority
    if &vrf_result_state.owner != bettor_account_info.key {
        msg!(
            "Expect VRF result account {} to be owned by user {}",
            vrf_result_account_info.key,
            bettor_account_info.key
        );
        return Err(ProgramError::from(BettingError::NoAuthority));
    }
    // update vrf result account
    vrf_result_state.marked_for_close = true;
    vrf_result_state.serialize(&mut &mut vrf_result_account_info.data.borrow_mut()[..])?;

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
        instructions::BettingInstruction,
        state::{
            game::{
                coinflip::{CoinFlipInput, CoinFlipSide},
                BetInput,
            },
            vrf_result::VrfResult,
        },
    };

    #[tokio::test]
    async fn test_vrf_result_mark_close_success() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Keypair::new();
        program_test.add_account(
            bettor.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_pda = Pubkey::new_unique();
        let bet_id = 0;
        let vrf_result_state = VrfResult::new(
            bettor.pubkey(),
            game_pda,
            bet_id,
            [0; 72],
            100,
            100,
            BetInput::CoinFlip {
                input: CoinFlipInput {
                    wager: 100,
                    side: CoinFlipSide::Head,
                },
            },
        );
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.pubkey().as_ref(), &bet_id.to_le_bytes()],
            &program_id,
        );
        program_test.add_account(
            vrf_result_pda,
            Account {
                lamports: Rent::default().minimum_balance(vrf_result_data_len),
                data: vrf_result_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::VrfResultMarkClose,
                vec![AccountMeta::new_readonly(bettor.pubkey(), true), AccountMeta::new(vrf_result_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result state should be updated
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.marked_for_close);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(0)")]
    async fn test_vrf_result_mark_close_err_no_authority() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Keypair::new();
        program_test.add_account(
            bettor.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let no_authority_user = Keypair::new();
        program_test.add_account(
            no_authority_user.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_pda = Pubkey::new_unique();
        let bet_id = 0;
        let vrf_result_state = VrfResult::new(
            bettor.pubkey(),
            game_pda,
            bet_id,
            [0; 72],
            100,
            100,
            BetInput::CoinFlip {
                input: CoinFlipInput {
                    wager: 100,
                    side: CoinFlipSide::Head,
                },
            },
        );
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.pubkey().as_ref(), &bet_id.to_le_bytes()],
            &program_id,
        );
        program_test.add_account(
            vrf_result_pda,
            Account {
                lamports: Rent::default().minimum_balance(vrf_result_data_len),
                data: vrf_result_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::VrfResultMarkClose,
                vec![
                    AccountMeta::new_readonly(no_authority_user.pubkey(), true),
                    AccountMeta::new(vrf_result_pda, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&no_authority_user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result state should be updated
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.marked_for_close);
    }
}

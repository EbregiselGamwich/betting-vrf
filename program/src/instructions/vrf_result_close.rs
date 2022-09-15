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
    checks::{check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    error::BettingError,
    state::{user_account::UserAccount, vrf_result::VrfResult, BettingAccount},
};

pub fn vrf_result_close(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: VrfResultClose");
    // get accounts
    let iter = &mut accounts.iter();

    let vrf_result_account_info = next_account_info(iter)?;
    let bettor_wallet_account_info = next_account_info(iter)?;
    let bettor_user_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let vrf_result_state = VrfResult::try_from_account_info(vrf_result_account_info)?;
    let mut bettor_user_account_state = UserAccount::try_from_account_info(bettor_user_account_info)?;
    // check accounts
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

    check_is_writable(bettor_wallet_account_info)?;
    check_pubkey_eq(bettor_wallet_account_info, &bettor_user_account_state.authority)?;
    check_pubkey_eq(bettor_wallet_account_info, &vrf_result_state.owner)?;

    check_is_writable(bettor_user_account_info)?;
    check_pda_cannonical_bump(
        bettor_user_account_info,
        &[b"UserAccount".as_ref(), bettor_user_account_state.authority.as_ref()],
    )?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;
    // check conditions
    if !vrf_result_state.marked_for_close {
        msg!("VRF result account {} is not marked for close", vrf_result_account_info.key);
        return Err(ProgramError::from(BettingError::VrfResultNotMarkedForClose));
    }
    if !vrf_result_state.is_fullfilled {
        msg!("VRF result account {} is not fullfileed", vrf_result_account_info.key);
        return Err(ProgramError::from(BettingError::VrfResultNotFullfilled));
    }
    if !vrf_result_state.is_used {
        msg!("VRF result account {} is not used", vrf_result_account_info.key);
        return Err(ProgramError::from(BettingError::VrfResultNotUsed));
    }
    // close vrf result account
    vrf_result_account_info.data.borrow_mut().fill(0);
    vrf_result_account_info.realloc(0, false)?;
    **bettor_wallet_account_info.lamports.borrow_mut() = bettor_wallet_account_info.lamports().checked_add(vrf_result_account_info.lamports()).unwrap();
    **vrf_result_account_info.lamports.borrow_mut() = 0;
    // update bettor user account
    bettor_user_account_state.active_vrf_results -= 1;
    bettor_user_account_state.serialize(&mut &mut bettor_user_account_info.data.borrow_mut()[..])?;
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
    use solana_sdk::{account::Account, signer::Signer, transaction::Transaction};

    use crate::{
        instructions::BettingInstruction,
        state::{
            game::{
                coinflip::{CoinFlipInput, CoinFlipSide},
                BetInput,
            },
            user_account::UserAccount,
            vrf_result::VrfResult,
        },
    };

    #[tokio::test]
    async fn test_vrf_result_close_success() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Pubkey::new_unique();
        program_test.add_account(
            bettor,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        bettor_user_account_state.active_vrf_results = 1;
        let bettor_user_account_data = bettor_user_account_state.try_to_vec().unwrap();
        let bettor_user_account_data_len = bettor_user_account_data.len();
        program_test.add_account(
            bettor_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(bettor_user_account_data_len),
                data: bettor_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let bet_id = 0;
        let game_pda = Pubkey::new_unique();
        let mut vrf_result_state = VrfResult::new(
            bettor,
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
        vrf_result_state.marked_for_close = true;
        vrf_result_state.is_fullfilled = true;
        vrf_result_state.is_used = true;
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) =
            Pubkey::find_program_address(&[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.as_ref(), &bet_id.to_le_bytes()], &program_id);
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
                &BettingInstruction::VrfResultClose,
                vec![
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(bettor, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result account should be closed
        assert!(banks_client.get_account(vrf_result_pda).await.unwrap().is_none());
        // the rent should be returned to the bettor
        let bettor_account = banks_client.get_account(bettor).await.unwrap().unwrap();
        assert_eq!(bettor_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(vrf_result_data_len));
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.active_vrf_results, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(12)")]
    async fn test_vrf_result_close_err_not_marked_for_close() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Pubkey::new_unique();
        program_test.add_account(
            bettor,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        bettor_user_account_state.active_vrf_results = 1;
        let bettor_user_account_data = bettor_user_account_state.try_to_vec().unwrap();
        let bettor_user_account_data_len = bettor_user_account_data.len();
        program_test.add_account(
            bettor_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(bettor_user_account_data_len),
                data: bettor_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let bet_id = 0;
        let game_pda = Pubkey::new_unique();
        let mut vrf_result_state = VrfResult::new(
            bettor,
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
        vrf_result_state.marked_for_close = false;
        vrf_result_state.is_fullfilled = true;
        vrf_result_state.is_used = true;
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) =
            Pubkey::find_program_address(&[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.as_ref(), &bet_id.to_le_bytes()], &program_id);
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
                &BettingInstruction::VrfResultClose,
                vec![
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(bettor, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result account should be closed
        assert!(banks_client.get_account(vrf_result_pda).await.unwrap().is_none());
        // the rent should be returned to the bettor
        let bettor_account = banks_client.get_account(bettor).await.unwrap().unwrap();
        assert_eq!(bettor_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(vrf_result_data_len));
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.active_vrf_results, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(6)")]
    async fn test_vrf_result_close_err_not_fullfilled_yet() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Pubkey::new_unique();
        program_test.add_account(
            bettor,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        bettor_user_account_state.active_vrf_results = 1;
        let bettor_user_account_data = bettor_user_account_state.try_to_vec().unwrap();
        let bettor_user_account_data_len = bettor_user_account_data.len();
        program_test.add_account(
            bettor_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(bettor_user_account_data_len),
                data: bettor_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let bet_id = 0;
        let game_pda = Pubkey::new_unique();
        let mut vrf_result_state = VrfResult::new(
            bettor,
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
        vrf_result_state.marked_for_close = true;
        vrf_result_state.is_fullfilled = false;
        vrf_result_state.is_used = true;
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) =
            Pubkey::find_program_address(&[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.as_ref(), &bet_id.to_le_bytes()], &program_id);
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
                &BettingInstruction::VrfResultClose,
                vec![
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(bettor, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result account should be closed
        assert!(banks_client.get_account(vrf_result_pda).await.unwrap().is_none());
        // the rent should be returned to the bettor
        let bettor_account = banks_client.get_account(bettor).await.unwrap().unwrap();
        assert_eq!(bettor_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(vrf_result_data_len));
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.active_vrf_results, 0);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(11)")]
    async fn test_vrf_result_close_not_used_yet() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let bettor = Pubkey::new_unique();
        program_test.add_account(
            bettor,
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        bettor_user_account_state.active_vrf_results = 1;
        let bettor_user_account_data = bettor_user_account_state.try_to_vec().unwrap();
        let bettor_user_account_data_len = bettor_user_account_data.len();
        program_test.add_account(
            bettor_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(bettor_user_account_data_len),
                data: bettor_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let bet_id = 0;
        let game_pda = Pubkey::new_unique();
        let mut vrf_result_state = VrfResult::new(
            bettor,
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
        vrf_result_state.marked_for_close = true;
        vrf_result_state.is_fullfilled = true;
        vrf_result_state.is_used = false;
        let vrf_result_data = vrf_result_state.try_to_vec().unwrap();
        let vrf_result_data_len = vrf_result_data.len();
        let (vrf_result_pda, _) =
            Pubkey::find_program_address(&[b"VrfResult".as_ref(), game_pda.as_ref(), bettor.as_ref(), &bet_id.to_le_bytes()], &program_id);
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
                &BettingInstruction::VrfResultClose,
                vec![
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(bettor, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result account should be closed
        assert!(banks_client.get_account(vrf_result_pda).await.unwrap().is_none());
        // the rent should be returned to the bettor
        let bettor_account = banks_client.get_account(bettor).await.unwrap().unwrap();
        assert_eq!(bettor_account.lamports, LAMPORTS_PER_SOL + Rent::default().minimum_balance(vrf_result_data_len));
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.active_vrf_results, 0);
    }
}

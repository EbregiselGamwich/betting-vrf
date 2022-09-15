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
    state::{vrf_result::VrfResult, BettingAccount},
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug)]
pub struct VrfResultFullfillArgs {
    pub beta: [u8; 64],
    pub pi: [u8; 80],
}

pub fn vrf_result_fullfill(_program_id: &Pubkey, accounts: &[AccountInfo], args: VrfResultFullfillArgs) -> ProgramResult {
    msg!("Instruction VrfResultFullfill");
    // get accounts
    let iter = &mut accounts.iter();

    let op_account_info = next_account_info(iter)?;
    let vrf_result_account_info = next_account_info(iter)?;

    let mut vrf_result_state = VrfResult::try_from_account_info(vrf_result_account_info)?;
    // check accounts
    check_is_signer(op_account_info)?;
    check_pubkey_eq(op_account_info, &OPERATOR_PUBKEY)?;

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

    // check conditions
    if vrf_result_state.is_fullfilled {
        msg!("VRF result account {} is already fullfilled", vrf_result_account_info.key);
        return Err(ProgramError::from(BettingError::VrfResultAlreadyFullfilled));
    }
    if vrf_result_state.is_used {
        msg!("VRF result account {} is already used", vrf_result_account_info.key);
        return Err(ProgramError::from(BettingError::VrfResultAlreadyUsed));
    }
    // update vrf result account
    vrf_result_state.is_fullfilled = true;
    vrf_result_state.beta = args.beta;
    vrf_result_state.pi = args.pi;
    vrf_result_state.serialize(&mut &mut vrf_result_account_info.data.borrow_mut()[..])?;

    Ok(())
}

#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use home::home_dir;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        rent::Rent,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::read_keypair_file, signer::Signer, transaction::Transaction};

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

    use super::VrfResultFullfillArgs;

    #[tokio::test]
    async fn test_vrf_result_fullfill_success() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let key_file_path = home_dir().unwrap().join(".config/solana/id.json");
        let operator = read_keypair_file(key_file_path).unwrap();
        program_test.add_account(
            operator.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_pda = Pubkey::new_unique();
        let bettor = Pubkey::new_unique();
        let bet_id = 0;
        let vrf_result_state = VrfResult::new(
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
                &BettingInstruction::VrfResultFullfill {
                    args: VrfResultFullfillArgs { beta: [1; 64], pi: [1; 80] },
                },
                vec![AccountMeta::new_readonly(operator.pubkey(), true), AccountMeta::new(vrf_result_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&operator, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result state should be updated
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.is_fullfilled);
        assert_eq!(vrf_result_state.beta, [1; 64]);
        assert_eq!(vrf_result_state.pi, [1; 80]);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(10)")]
    async fn test_vrf_result_fullfill_err_already_fullfilled() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let key_file_path = home_dir().unwrap().join(".config/solana/id.json");
        let operator = read_keypair_file(key_file_path).unwrap();
        program_test.add_account(
            operator.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_pda = Pubkey::new_unique();
        let bettor = Pubkey::new_unique();
        let bet_id = 0;
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
        vrf_result_state.is_fullfilled = true;
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
                &BettingInstruction::VrfResultFullfill {
                    args: VrfResultFullfillArgs { beta: [1; 64], pi: [1; 80] },
                },
                vec![AccountMeta::new_readonly(operator.pubkey(), true), AccountMeta::new(vrf_result_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&operator, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result state should be updated
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.is_fullfilled);
        assert_eq!(vrf_result_state.beta, [1; 64]);
        assert_eq!(vrf_result_state.pi, [1; 80]);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(7)")]
    async fn test_vrf_result_fullfill_err_already_used() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let key_file_path = home_dir().unwrap().join(".config/solana/id.json");
        let operator = read_keypair_file(key_file_path).unwrap();
        program_test.add_account(
            operator.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_pda = Pubkey::new_unique();
        let bettor = Pubkey::new_unique();
        let bet_id = 0;
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
                &BettingInstruction::VrfResultFullfill {
                    args: VrfResultFullfillArgs { beta: [1; 64], pi: [1; 80] },
                },
                vec![AccountMeta::new_readonly(operator.pubkey(), true), AccountMeta::new(vrf_result_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&operator, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the vrf result state should be updated
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.is_fullfilled);
        assert_eq!(vrf_result_state.beta, [1; 64]);
        assert_eq!(vrf_result_state.pi, [1; 80]);
    }
}

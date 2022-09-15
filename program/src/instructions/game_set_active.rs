use borsh::{BorshDeserialize, BorshSerialize};
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
    state::{game::Game, BettingAccount},
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct GameSetActiveArgs {
    pub is_active: bool,
}
pub fn game_set_active(_program_id: &Pubkey, accounts: &[AccountInfo], args: GameSetActiveArgs) -> ProgramResult {
    msg!("Instruction: GameSetActive");
    // get accounts
    let iter = &mut accounts.iter();

    let host_account_info = next_account_info(iter)?;
    let game_account_info = next_account_info(iter)?;

    let mut game_state = Game::try_from_account_info(game_account_info)?;
    // check accounts
    check_is_signer(host_account_info)?;

    check_is_writable(game_account_info)?;
    let common_config_vec = game_state.common_config.try_to_vec()?;
    let game_type_config_vec = game_state.game_type_config.try_to_vec()?;
    check_pda_cannonical_bump(
        game_account_info,
        &[b"Game".as_ref(), common_config_vec.as_slice(), game_type_config_vec.as_slice()],
    )?;
    // check authority
    if &game_state.host != host_account_info.key {
        msg!("Expect account {} to be the host of the game {}", host_account_info.key, game_account_info.key);
        return Err(ProgramError::from(BettingError::NoAuthority));
    }
    // update game state
    game_state.is_active = args.is_active;
    game_state.serialize(&mut &mut game_account_info.data.borrow_mut()[..])?;

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
        state::game::{crash::CrashConfig, Game, GameTypeConfig},
    };

    use super::GameSetActiveArgs;

    #[tokio::test]
    async fn test_game_set_active_success() {
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

        let game_state = Game::new(
            user.pubkey(),
            1000,
            10000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
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
                &BettingInstruction::GameSetActive {
                    args: GameSetActiveArgs { is_active: false },
                },
                vec![AccountMeta::new_readonly(user.pubkey(), true), AccountMeta::new(game_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the game state should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert!(!game_state.is_active);
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(0)")]
    async fn test_game_set_active_err_no_authority() {
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

        let no_authority_user = Keypair::new();
        program_test.add_account(
            no_authority_user.pubkey(),
            Account {
                lamports: LAMPORTS_PER_SOL,
                ..Default::default()
            },
        );

        let game_state = Game::new(
            user.pubkey(),
            1000,
            10000,
            GameTypeConfig::Crash {
                config: CrashConfig {
                    multiplier_straight_one_possibility: 100,
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
                &BettingInstruction::GameSetActive {
                    args: GameSetActiveArgs { is_active: false },
                },
                vec![AccountMeta::new_readonly(no_authority_user.pubkey(), true), AccountMeta::new(game_pda, false)],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&no_authority_user, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // the game state should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert!(!game_state.is_active);
    }
}

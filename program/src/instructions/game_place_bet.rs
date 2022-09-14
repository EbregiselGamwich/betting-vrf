use std::convert::TryInto;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::{self, Sysvar},
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    error::BettingError,
    state::{
        game::{BetInput, Game},
        stats::Stats,
        user_account::UserAccount,
        vrf_result::VrfResult,
        BettingAccount,
    },
};
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy)]
pub struct GamePlaceBetArgs {
    pub bet_input: BetInput,
}

pub fn game_place_bet(program_id: &Pubkey, accounts: &[AccountInfo], args: GamePlaceBetArgs) -> ProgramResult {
    msg!("Instruction: GamePlaceBet");
    // get accounts
    let iter = &mut accounts.iter();

    let bettor_account_info = next_account_info(iter)?;
    let bettor_user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let game_account_info = next_account_info(iter)?;
    let host_user_account_info = next_account_info(iter)?;
    let vrf_result_account_info = next_account_info(iter)?;
    let slot_hashes_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut bettor_user_account_state = UserAccount::try_from_account_info(bettor_user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    let mut game_account_state = Game::try_from_account_info(game_account_info)?;
    let mut host_user_account_state = UserAccount::try_from_account_info(host_user_account_info)?;
    // check accounts
    check_is_signer(bettor_account_info)?;

    check_is_writable(bettor_user_account_info)?;
    check_pda_cannonical_bump(
        bettor_user_account_info,
        &[b"UserAccount".as_ref(), bettor_user_account_state.authority.as_ref()],
    )?;
    check_pubkey_eq(bettor_account_info, &bettor_user_account_state.authority)?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_is_writable(game_account_info)?;
    let game_common_config_vec = game_account_state.common_config.try_to_vec()?;
    let game_type_config_vec = game_account_state.game_type_config.try_to_vec()?;
    check_pda_cannonical_bump(
        game_account_info,
        &[b"Game".as_ref(), game_common_config_vec.as_slice(), game_type_config_vec.as_slice()],
    )?;

    check_is_writable(host_user_account_info)?;
    check_pda_cannonical_bump(host_user_account_info, &[b"UserAccount".as_ref(), host_user_account_state.authority.as_ref()])?;
    assert_eq!(&host_user_account_state.authority, &game_account_state.host);

    check_is_writable(vrf_result_account_info)?;
    let vrf_result_pda_bump = check_pda_cannonical_bump(
        vrf_result_account_info,
        &[
            b"VrfResult".as_ref(),
            game_account_info.key.as_ref(),
            bettor_account_info.key.as_ref(),
            &bettor_user_account_state.total_bets.to_le_bytes(),
        ],
    )?;

    check_pubkey_eq(slot_hashes_account_info, &sysvar::slot_hashes::ID)?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;

    // check game is active
    if !game_account_state.is_active {
        msg!("Game {} is not active", game_account_info.key);
        return Err(ProgramError::from(BettingError::GameNotActive));
    }
    // check bet input
    let dyn_bet_input = args.bet_input.get_dyn_input();
    dyn_bet_input.check_bet_input(&game_account_state)?;
    // check bettor balance
    let bettor_lamports_to_lock = dyn_bet_input.check_bettor_balance(&game_account_state, &bettor_user_account_state)?;
    // check host balacne
    let host_lamports_to_lock = dyn_bet_input.check_host_balance(&game_account_state, &host_user_account_state)?;

    // update bettor user account
    let bet_id = bettor_user_account_state.total_bets;
    bettor_user_account_state.total_bets += 1;
    bettor_user_account_state.active_vrf_results += 1;
    bettor_user_account_state.current_lamports -= bettor_lamports_to_lock;
    bettor_user_account_state.serialize(&mut &mut bettor_user_account_info.data.borrow_mut()[..])?;
    // update stats account
    stats_account_state.total_bets += 1;
    stats_account_state.total_wager += bettor_lamports_to_lock;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;
    // update game account
    game_account_state.unresolved_vrf_result += 1;
    game_account_state.total_lamports_in += bettor_lamports_to_lock;
    game_account_state.serialize(&mut &mut game_account_info.data.borrow_mut()[..])?;
    // update host user account
    host_user_account_state.current_lamports -= host_lamports_to_lock;
    host_user_account_state.serialize(&mut &mut host_user_account_info.data.borrow_mut()[..])?;
    // create vrf result account
    let mut alpha = [0u8; 72];
    let now = Clock::get()?.unix_timestamp;
    alpha[0..8].copy_from_slice(&now.to_le_bytes());
    alpha[8..40].copy_from_slice(bettor_account_info.key.as_ref());
    alpha[40..72].copy_from_slice(&slot_hashes_account_info.data.borrow()[16..48]);
    let vrf_result_state = VrfResult::new(
        *bettor_account_info.key,
        *game_account_info.key,
        bet_id,
        alpha,
        bettor_lamports_to_lock,
        host_lamports_to_lock,
        args.bet_input,
    );
    let vrf_result_data = vrf_result_state.try_to_vec()?;
    let vrf_result_data_len = vrf_result_data.len();
    let vrf_result_pda_signer_seeds = &[
        b"VrfResult".as_ref(),
        game_account_info.key.as_ref(),
        bettor_account_info.key.as_ref(),
        &bet_id.to_le_bytes(),
        &[vrf_result_pda_bump],
    ];
    let min_rent = Rent::get()?.minimum_balance(vrf_result_data_len);
    let vrf_result_create_ix = system_instruction::create_account(
        bettor_account_info.key,
        vrf_result_account_info.key,
        min_rent,
        vrf_result_data_len.try_into().unwrap(),
        program_id,
    );
    invoke_signed(
        &vrf_result_create_ix,
        &[bettor_account_info.clone(), vrf_result_account_info.clone()],
        &[vrf_result_pda_signer_seeds],
    )?;
    vrf_result_account_info.data.borrow_mut().copy_from_slice(&vrf_result_data);

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
        system_program, sysvar,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

    use crate::{
        instructions::BettingInstruction,
        state::{
            game::{
                coinflip::{CoinFlipConfig, CoinFlipInput, CoinFlipSide},
                BetInput, Game, GameTypeConfig,
            },
            stats::Stats,
            user_account::UserAccount,
            vrf_result::VrfResult,
            StateAccountType,
        },
    };

    use super::GamePlaceBetArgs;

    #[tokio::test]
    async fn test_game_place_bet_success() {
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

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.pubkey().as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor.pubkey(), Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
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

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        let host_user_account_data = host_user_account_state.try_to_vec().unwrap();
        let host_user_account_data_len = host_user_account_data.len();
        program_test.add_account(
            host_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(host_user_account_data_len),
                data: host_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let game_state = Game::new(
            host,
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
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

        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[
                b"VrfResult".as_ref(),
                game_pda.as_ref(),
                bettor.pubkey().as_ref(),
                &bettor_user_account_state.total_bets.to_le_bytes(),
            ],
            &program_id,
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GamePlaceBet {
                    args: GamePlaceBetArgs {
                        bet_input: BetInput::CoinFlip {
                            input: CoinFlipInput {
                                wager: 2000,
                                side: CoinFlipSide::Head,
                            },
                        },
                    },
                },
                vec![
                    AccountMeta::new(bettor.pubkey(), true),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.total_bets, 1);
        assert_eq!(bettor_user_account_state.active_vrf_results, 1);
        assert_eq!(bettor_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_bets, 1);
        assert_eq!(stats_state.total_wager, 2000);
        // the game account should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 1);
        assert_eq!(game_state.total_lamports_in, 2000);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000 * 9900 / 10000);
        // the vrf result account should be created
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert_eq!(vrf_result_state.account_type, StateAccountType::Vrf);
        assert!(!vrf_result_state.is_fullfilled);
        assert!(!vrf_result_state.is_used);
        assert!(!vrf_result_state.marked_for_close);
        assert_eq!(vrf_result_state.owner, bettor.pubkey());
        assert_eq!(vrf_result_state.game, game_pda);
        assert_eq!(vrf_result_state.bet_id, 0);
        assert_eq!(&vrf_result_state.alpha[8..40], bettor.pubkey().as_ref());
        assert_eq!(vrf_result_state.beta, [0; 64]);
        assert_eq!(vrf_result_state.pi, [0; 80]);
        assert_eq!(vrf_result_state.locked_bettor_lamports, 2000);
        assert_eq!(vrf_result_state.locked_host_lamports, 2000 * 9900 / 10000);
        if let BetInput::CoinFlip { input } = vrf_result_state.bet_input {
            assert_eq!(input.wager, 2000);
            assert_eq!(input.side, CoinFlipSide::Head);
        } else {
            panic!()
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Custom(9)")]
    async fn test_game_place_bet_err_game_not_active() {
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

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.pubkey().as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor.pubkey(), Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
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

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        let host_user_account_data = host_user_account_state.try_to_vec().unwrap();
        let host_user_account_data_len = host_user_account_data.len();
        program_test.add_account(
            host_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(host_user_account_data_len),
                data: host_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let mut game_state = Game::new(
            host,
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
                },
            },
        );
        game_state.is_active = false;
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

        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[
                b"VrfResult".as_ref(),
                game_pda.as_ref(),
                bettor.pubkey().as_ref(),
                &bettor_user_account_state.total_bets.to_le_bytes(),
            ],
            &program_id,
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GamePlaceBet {
                    args: GamePlaceBetArgs {
                        bet_input: BetInput::CoinFlip {
                            input: CoinFlipInput {
                                wager: 2000,
                                side: CoinFlipSide::Head,
                            },
                        },
                    },
                },
                vec![
                    AccountMeta::new(bettor.pubkey(), true),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.total_bets, 1);
        assert_eq!(bettor_user_account_state.active_vrf_results, 1);
        assert_eq!(bettor_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_bets, 1);
        assert_eq!(stats_state.total_wager, 2000);
        // the game account should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 1);
        assert_eq!(game_state.total_lamports_in, 2000);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000 * 9900 / 10000);
        // the vrf result account should be created
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert_eq!(vrf_result_state.account_type, StateAccountType::Vrf);
        assert!(!vrf_result_state.is_fullfilled);
        assert!(!vrf_result_state.is_used);
        assert!(!vrf_result_state.marked_for_close);
        assert_eq!(vrf_result_state.owner, bettor.pubkey());
        assert_eq!(vrf_result_state.game, game_pda);
        assert_eq!(vrf_result_state.bet_id, 0);
        assert_eq!(&vrf_result_state.alpha[8..40], bettor.pubkey().as_ref());
        assert_eq!(vrf_result_state.beta, [0; 64]);
        assert_eq!(vrf_result_state.pi, [0; 80]);
        assert_eq!(vrf_result_state.locked_bettor_lamports, 2000);
        assert_eq!(vrf_result_state.locked_host_lamports, 2000 * 9900 / 10000);
        if let BetInput::CoinFlip { input } = vrf_result_state.bet_input {
            assert_eq!(input.wager, 2000);
            assert_eq!(input.side, CoinFlipSide::Head);
        } else {
            panic!()
        }
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidArgument")]
    async fn test_game_place_bet_err_invalid_bet_input() {
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

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.pubkey().as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor.pubkey(), Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
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

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        let host_user_account_data = host_user_account_state.try_to_vec().unwrap();
        let host_user_account_data_len = host_user_account_data.len();
        program_test.add_account(
            host_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(host_user_account_data_len),
                data: host_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let game_state = Game::new(
            host,
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
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

        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[
                b"VrfResult".as_ref(),
                game_pda.as_ref(),
                bettor.pubkey().as_ref(),
                &bettor_user_account_state.total_bets.to_le_bytes(),
            ],
            &program_id,
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GamePlaceBet {
                    args: GamePlaceBetArgs {
                        bet_input: BetInput::CoinFlip {
                            input: CoinFlipInput {
                                wager: 2000000,
                                side: CoinFlipSide::Head,
                            },
                        },
                    },
                },
                vec![
                    AccountMeta::new(bettor.pubkey(), true),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.total_bets, 1);
        assert_eq!(bettor_user_account_state.active_vrf_results, 1);
        assert_eq!(bettor_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_bets, 1);
        assert_eq!(stats_state.total_wager, 2000);
        // the game account should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 1);
        assert_eq!(game_state.total_lamports_in, 2000);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000 * 9900 / 10000);
        // the vrf result account should be created
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert_eq!(vrf_result_state.account_type, StateAccountType::Vrf);
        assert!(!vrf_result_state.is_fullfilled);
        assert!(!vrf_result_state.is_used);
        assert!(!vrf_result_state.marked_for_close);
        assert_eq!(vrf_result_state.owner, bettor.pubkey());
        assert_eq!(vrf_result_state.game, game_pda);
        assert_eq!(vrf_result_state.bet_id, 0);
        assert_eq!(&vrf_result_state.alpha[8..40], bettor.pubkey().as_ref());
        assert_eq!(vrf_result_state.beta, [0; 64]);
        assert_eq!(vrf_result_state.pi, [0; 80]);
        assert_eq!(vrf_result_state.locked_bettor_lamports, 2000);
        assert_eq!(vrf_result_state.locked_host_lamports, 2000 * 9900 / 10000);
        if let BetInput::CoinFlip { input } = vrf_result_state.bet_input {
            assert_eq!(input.wager, 2000);
            assert_eq!(input.side, CoinFlipSide::Head);
        } else {
            panic!()
        }
    }

    #[tokio::test]
    #[should_panic(expected = "InsufficientFunds")]
    async fn test_game_place_bet_err_bettor_not_enough_money() {
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

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.pubkey().as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor.pubkey(), Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = 1;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
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

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = LAMPORTS_PER_SOL;
        let host_user_account_data = host_user_account_state.try_to_vec().unwrap();
        let host_user_account_data_len = host_user_account_data.len();
        program_test.add_account(
            host_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(host_user_account_data_len),
                data: host_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let game_state = Game::new(
            host,
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
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

        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[
                b"VrfResult".as_ref(),
                game_pda.as_ref(),
                bettor.pubkey().as_ref(),
                &bettor_user_account_state.total_bets.to_le_bytes(),
            ],
            &program_id,
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GamePlaceBet {
                    args: GamePlaceBetArgs {
                        bet_input: BetInput::CoinFlip {
                            input: CoinFlipInput {
                                wager: 2000,
                                side: CoinFlipSide::Head,
                            },
                        },
                    },
                },
                vec![
                    AccountMeta::new(bettor.pubkey(), true),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.total_bets, 1);
        assert_eq!(bettor_user_account_state.active_vrf_results, 1);
        assert_eq!(bettor_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_bets, 1);
        assert_eq!(stats_state.total_wager, 2000);
        // the game account should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 1);
        assert_eq!(game_state.total_lamports_in, 2000);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000 * 9900 / 10000);
        // the vrf result account should be created
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert_eq!(vrf_result_state.account_type, StateAccountType::Vrf);
        assert!(!vrf_result_state.is_fullfilled);
        assert!(!vrf_result_state.is_used);
        assert!(!vrf_result_state.marked_for_close);
        assert_eq!(vrf_result_state.owner, bettor.pubkey());
        assert_eq!(vrf_result_state.game, game_pda);
        assert_eq!(vrf_result_state.bet_id, 0);
        assert_eq!(&vrf_result_state.alpha[8..40], bettor.pubkey().as_ref());
        assert_eq!(vrf_result_state.beta, [0; 64]);
        assert_eq!(vrf_result_state.pi, [0; 80]);
        assert_eq!(vrf_result_state.locked_bettor_lamports, 2000);
        assert_eq!(vrf_result_state.locked_host_lamports, 2000 * 9900 / 10000);
        if let BetInput::CoinFlip { input } = vrf_result_state.bet_input {
            assert_eq!(input.wager, 2000);
            assert_eq!(input.side, CoinFlipSide::Head);
        } else {
            panic!()
        }
    }

    #[tokio::test]
    #[should_panic(expected = "InsufficientFunds")]
    async fn test_game_place_bet_err_host_not_enough_money() {
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

        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.pubkey().as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor.pubkey(), Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = LAMPORTS_PER_SOL;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
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

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = 1;
        let host_user_account_data = host_user_account_state.try_to_vec().unwrap();
        let host_user_account_data_len = host_user_account_data.len();
        program_test.add_account(
            host_user_account_pda,
            Account {
                lamports: Rent::default().minimum_balance(host_user_account_data_len),
                data: host_user_account_data,
                owner: program_id,
                ..Default::default()
            },
        );

        let game_state = Game::new(
            host,
            1000,
            10000,
            GameTypeConfig::CoinFlip {
                config: CoinFlipConfig {
                    host_probability_advantage: 100,
                    payout_rate: 9900,
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

        let (vrf_result_pda, _) = Pubkey::find_program_address(
            &[
                b"VrfResult".as_ref(),
                game_pda.as_ref(),
                bettor.pubkey().as_ref(),
                &bettor_user_account_state.total_bets.to_le_bytes(),
            ],
            &program_id,
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_borsh(
                program_id,
                &BettingInstruction::GamePlaceBet {
                    args: GamePlaceBetArgs {
                        bet_input: BetInput::CoinFlip {
                            input: CoinFlipInput {
                                wager: 2000,
                                side: CoinFlipSide::Head,
                            },
                        },
                    },
                },
                vec![
                    AccountMeta::new(bettor.pubkey(), true),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&bettor, &payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.total_bets, 1);
        assert_eq!(bettor_user_account_state.active_vrf_results, 1);
        assert_eq!(bettor_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_bets, 1);
        assert_eq!(stats_state.total_wager, 2000);
        // the game account should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 1);
        assert_eq!(game_state.total_lamports_in, 2000);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, LAMPORTS_PER_SOL - 2000 * 9900 / 10000);
        // the vrf result account should be created
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert_eq!(vrf_result_state.account_type, StateAccountType::Vrf);
        assert!(!vrf_result_state.is_fullfilled);
        assert!(!vrf_result_state.is_used);
        assert!(!vrf_result_state.marked_for_close);
        assert_eq!(vrf_result_state.owner, bettor.pubkey());
        assert_eq!(vrf_result_state.game, game_pda);
        assert_eq!(vrf_result_state.bet_id, 0);
        assert_eq!(&vrf_result_state.alpha[8..40], bettor.pubkey().as_ref());
        assert_eq!(vrf_result_state.beta, [0; 64]);
        assert_eq!(vrf_result_state.pi, [0; 80]);
        assert_eq!(vrf_result_state.locked_bettor_lamports, 2000);
        assert_eq!(vrf_result_state.locked_host_lamports, 2000 * 9900 / 10000);
        if let BetInput::CoinFlip { input } = vrf_result_state.bet_input {
            assert_eq!(input.wager, 2000);
            assert_eq!(input.side, CoinFlipSide::Head);
        } else {
            panic!()
        }
    }
}

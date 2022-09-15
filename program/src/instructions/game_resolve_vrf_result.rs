use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

use crate::{
    checks::{check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    state::{game::Game, stats::Stats, user_account::UserAccount, vrf_result::VrfResult, BettingAccount},
};

pub fn game_resolve_vrf_result(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Instruction: GameResolveVrfResult");
    // get accounts
    let iter = &mut accounts.iter();

    let game_account_info = next_account_info(iter)?;
    let vrf_result_account_info = next_account_info(iter)?;
    let host_user_account_info = next_account_info(iter)?;
    let bettor_user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;

    let mut game_state = Game::try_from_account_info(game_account_info)?;
    let mut vrf_result_state = VrfResult::try_from_account_info(vrf_result_account_info)?;
    let mut host_user_account_state = UserAccount::try_from_account_info(host_user_account_info)?;
    let mut bettor_user_account_state = UserAccount::try_from_account_info(bettor_user_account_info)?;
    let mut stats_state = Stats::try_from_account_info(stats_account_info)?;

    // check accounts
    check_is_writable(game_account_info)?;
    let game_common_config_vec = game_state.common_config.try_to_vec()?;
    let game_type_config_vec = game_state.game_type_config.try_to_vec()?;
    check_pda_cannonical_bump(
        game_account_info,
        &[b"Game".as_ref(), game_common_config_vec.as_slice(), game_type_config_vec.as_slice()],
    )?;

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
    check_pubkey_eq(game_account_info, &vrf_result_state.game)?;
    assert_eq!(&bettor_user_account_state.authority, &vrf_result_state.owner);

    check_is_writable(host_user_account_info)?;
    check_pda_cannonical_bump(host_user_account_info, &[b"UserAccount".as_ref(), host_user_account_state.authority.as_ref()])?;

    check_is_writable(bettor_user_account_info)?;
    check_pda_cannonical_bump(
        bettor_user_account_info,
        &[b"UserAccount".as_ref(), bettor_user_account_state.authority.as_ref()],
    )?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    // bet result
    let game_type_dyn = game_state.game_type_config.get_dyn_config();
    if game_type_dyn.process_vrf_result(&vrf_result_state)? {
        // bettor won
        // update game account
        game_state.unresolved_vrf_result -= 1;
        game_state.total_lamports_out += vrf_result_state.locked_host_lamports;
        game_state.serialize(&mut &mut game_account_info.data.borrow_mut()[..])?;
        // update vrf result account
        vrf_result_state.is_used = true;
        vrf_result_state.serialize(&mut &mut vrf_result_account_info.data.borrow_mut()[..])?;
        // update bettor user account
        bettor_user_account_state.current_lamports += vrf_result_state.locked_bettor_lamports; // return locked wager
        bettor_user_account_state.current_lamports += vrf_result_state.locked_host_lamports; // get lamports won from the host
        bettor_user_account_state.serialize(&mut &mut bettor_user_account_info.data.borrow_mut()[..])?;
        // update stasts account
        stats_state.total_lamports_won_by_bettors += vrf_result_state.locked_host_lamports;
        stats_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;
    } else {
        // bettor lost
        // update game account
        game_state.unresolved_vrf_result -= 1;
        game_state.serialize(&mut &mut game_account_info.data.borrow_mut()[..])?;
        // update vrf result account
        vrf_result_state.is_used = true;
        vrf_result_state.serialize(&mut &mut vrf_result_account_info.data.borrow_mut()[..])?;
        // update host user account
        host_user_account_state.current_lamports += vrf_result_state.locked_host_lamports;
        host_user_account_state.current_lamports += vrf_result_state.locked_bettor_lamports;
        host_user_account_state.serialize(&mut &mut host_user_account_info.data.borrow_mut()[..])?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        rent::Rent,
    };
    use solana_program_test::{tokio, ProgramTest};
    use solana_sdk::{account::Account, signer::Signer, transaction::Transaction};

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
        },
    };

    #[tokio::test]
    async fn test_game_resolve_vrf_result_success_bettor_won() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = 0;
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

        let bettor = Pubkey::new_unique();
        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = 0;
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
        game_state.unresolved_vrf_result = 1;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
        stats_state.total_bets = 1;
        stats_state.total_wager = 2000;
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

        let bet_id = 0;
        let mut vrf_result_state = VrfResult::new(
            bettor,
            game_pda,
            bet_id,
            [0; 72],
            2000,
            2000 * 9900 / 10000,
            BetInput::CoinFlip {
                input: CoinFlipInput {
                    wager: 2000,
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
                &BettingInstruction::GameResolveVrfResult,
                vec![
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the game pda should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 0);
        assert_eq!(game_state.total_lamports_out, 2000 * 9900 / 10000);
        // the vrf result should be update
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.is_used);
        // the bettor user account should be updated
        let bettor_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(bettor_user_account_pda).await.unwrap();
        assert_eq!(bettor_user_account_state.current_lamports, 2000 + 2000 * 9900 / 10000);
        // the stats account should be updated
        let stats_state: Stats = banks_client.get_account_data_with_borsh(stats_pda).await.unwrap();
        assert_eq!(stats_state.total_lamports_won_by_bettors, 2000 * 9900 / 10000);
    }

    #[tokio::test]
    async fn test_game_resolve_vrf_result_success_bettor_lost() {
        let program_id = crate::id();
        let mut program_test = ProgramTest::new("vrf_betting", program_id, None);

        let host = Pubkey::new_unique();
        let (host_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), host.as_ref()], &program_id);
        let mut host_user_account_state = UserAccount::new(host, None, None);
        host_user_account_state.current_lamports = 0;
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

        let bettor = Pubkey::new_unique();
        let referral = Pubkey::new_unique();
        let (bettor_user_account_pda, _) = Pubkey::find_program_address(&[b"UserAccount".as_ref(), bettor.as_ref()], &program_id);
        let mut bettor_user_account_state = UserAccount::new(bettor, Some(referral), Some("Bettor".to_string()));
        bettor_user_account_state.current_lamports = 0;
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
        game_state.unresolved_vrf_result = 1;
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

        let (stats_pda, _) = Pubkey::find_program_address(&[b"Stats".as_ref()], &program_id);
        let mut stats_state = Stats::new();
        stats_state.total_users = 1;
        stats_state.total_games = 1;
        stats_state.total_bets = 1;
        stats_state.total_wager = 2000;
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

        let bet_id = 0;
        let mut vrf_result_state = VrfResult::new(
            bettor,
            game_pda,
            bet_id,
            [0; 72],
            2000,
            2000 * 9900 / 10000,
            BetInput::CoinFlip {
                input: CoinFlipInput {
                    wager: 2000,
                    side: CoinFlipSide::Head,
                },
            },
        );
        vrf_result_state.marked_for_close = true;
        vrf_result_state.is_fullfilled = true;
        vrf_result_state.is_used = false;
        vrf_result_state.beta[0..16].copy_from_slice(&8000_u128.to_le_bytes());
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
                &BettingInstruction::GameResolveVrfResult,
                vec![
                    AccountMeta::new(game_pda, false),
                    AccountMeta::new(vrf_result_pda, false),
                    AccountMeta::new(host_user_account_pda, false),
                    AccountMeta::new(bettor_user_account_pda, false),
                    AccountMeta::new(stats_pda, false),
                ],
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
        // the game pda should be updated
        let game_state: Game = banks_client.get_account_data_with_borsh(game_pda).await.unwrap();
        assert_eq!(game_state.unresolved_vrf_result, 0);
        // the vrf result should be update
        let vrf_result_state: VrfResult = banks_client.get_account_data_with_borsh(vrf_result_pda).await.unwrap();
        assert!(vrf_result_state.is_used);
        // the host user account should be updated
        let host_user_account_state: UserAccount = banks_client.get_account_data_with_borsh(host_user_account_pda).await.unwrap();
        assert_eq!(host_user_account_state.current_lamports, 2000 + 2000 * 9900 / 10000);
    }
}

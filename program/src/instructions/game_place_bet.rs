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

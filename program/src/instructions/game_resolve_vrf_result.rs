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

pub fn game_resolve_vrf_result(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
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
    } else {
        // bettor lost
    }
    Ok(())
}

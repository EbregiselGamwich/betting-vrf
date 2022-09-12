use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
    system_instruction, system_program,
};

use crate::{
    checks::{check_is_signer, check_is_writable, check_pda_cannonical_bump, check_pubkey_eq},
    state::{stats::Stats, user_account::UserAccount, BettingAccount},
};
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserAccountDepositArgs {
    pub lamports: u64,
}
pub fn user_account_deposit(_program_id: &Pubkey, accounts: &[AccountInfo], args: UserAccountDepositArgs) -> ProgramResult {
    msg!("Instruction: UserAccountDeposit");
    // get accounts
    let iter = &mut accounts.iter();

    let depositor_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let system_program_account_info = next_account_info(iter)?;

    let mut user_account_state = UserAccount::try_from_account_info(user_account_info)?;
    let mut stats_account_state = Stats::try_from_account_info(stats_account_info)?;
    // check accounts
    check_is_signer(depositor_account_info)?;
    check_is_writable(depositor_account_info)?;

    check_is_writable(user_account_info)?;
    check_pda_cannonical_bump(user_account_info, &[b"UserAccount".as_ref(), user_account_state.authority.as_ref()])?;

    check_is_writable(stats_account_info)?;
    check_pda_cannonical_bump(stats_account_info, &[b"Stats".as_ref()])?;

    check_pubkey_eq(system_program_account_info, &system_program::ID)?;

    // transfer lamports
    let lamports_transfer_ix = system_instruction::transfer(depositor_account_info.key, stats_account_info.key, args.lamports);
    invoke(&lamports_transfer_ix, &[depositor_account_info.clone(), stats_account_info.clone()])?;

    // update user account
    user_account_state.current_lamports += args.lamports;
    user_account_state.serialize(&mut &mut user_account_info.data.borrow_mut()[..])?;

    // update stats account
    stats_account_state.total_lamports_deposited += args.lamports;
    stats_account_state.serialize(&mut &mut stats_account_info.data.borrow_mut()[..])?;

    Ok(())
}

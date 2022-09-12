use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct UserAccountWithdrawArgs {
    pub lamports: u64,
}
pub fn user_account_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], args: UserAccountWithdrawArgs) -> ProgramResult {
    msg!("Instruction: UserAccountWithdraw");
    // get accounts
    let iter = &mut accounts.iter();

    let user_wallet_account_info = next_account_info(iter)?;
    let user_account_info = next_account_info(iter)?;
    let stats_account_info = next_account_info(iter)?;
    let op_account_info = next_account_info(iter)?;
    Ok(())
}

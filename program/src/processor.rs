use crate::instructions::stats_account_create::stats_account_create;
use crate::instructions::user_account_close::user_account_close;
use crate::instructions::user_account_deposit::user_account_deposit;
use crate::instructions::user_account_withdraw::user_account_withdraw;
use crate::instructions::{user_account_create::user_account_create, BettingInstruction};
use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub struct Processor;
impl Processor {
    pub fn process_instruction(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction: BettingInstruction = BettingInstruction::try_from_slice(instruction_data)?;
        match instruction {
            BettingInstruction::StatsAccountCreate => stats_account_create(program_id, accounts),
            BettingInstruction::UserAccountCreate { args } => user_account_create(program_id, accounts, args),
            BettingInstruction::UserAccountDeposit { args } => user_account_deposit(program_id, accounts, args),
            BettingInstruction::UserAccountWithdraw { args } => user_account_withdraw(program_id, accounts, args),
            BettingInstruction::UserAccountClose => user_account_close(program_id, accounts),
        }
    }
}

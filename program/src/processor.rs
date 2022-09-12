use crate::instructions::{mint_create::mint_create, stats_account_create::stats_account_create};
use crate::instructions::{user_account_create::user_account_create, BettingInstruction};
use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub struct Processor;
impl Processor {
    pub fn process_instruction(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction: BettingInstruction = BettingInstruction::try_from_slice(instruction_data)?;
        match instruction {
            BettingInstruction::StatsAccountCreate => stats_account_create(program_id, accounts),
            BettingInstruction::MintCreate => mint_create(program_id, accounts),
            BettingInstruction::UserAccountCreate { args } => user_account_create(program_id, accounts, args),
        }
    }
}

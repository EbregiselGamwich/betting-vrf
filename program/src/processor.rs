use crate::instructions::game_close::game_close;
use crate::instructions::game_resolve_vrf_result::game_resolve_vrf_result;
use crate::instructions::user_account_withdraw::user_account_withdraw;
use crate::instructions::vrf_result_close::vrf_result_close;
use crate::instructions::vrf_result_fullfill::vrf_result_fullfill;
use crate::instructions::vrf_result_mark_close::vrf_result_mark_close;
use crate::instructions::{game_create::game_create, stats_account_create::stats_account_create};
use crate::instructions::{game_place_bet::game_place_bet, user_account_deposit::user_account_deposit};
use crate::instructions::{game_set_active::game_set_active, user_account_close::user_account_close};
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
            BettingInstruction::GameCreate { args } => game_create(program_id, accounts, args),
            BettingInstruction::GameSetActive { args } => game_set_active(program_id, accounts, args),
            BettingInstruction::GamePlaceBet { args } => game_place_bet(program_id, accounts, args),
            BettingInstruction::VrfResultFullfill { args } => vrf_result_fullfill(program_id, accounts, args),
            BettingInstruction::VrfResultMarkClose => vrf_result_mark_close(program_id, accounts),
            BettingInstruction::VrfResultClose => vrf_result_close(program_id, accounts),
            BettingInstruction::GameResolveVrfResult => game_resolve_vrf_result(program_id, accounts),
            BettingInstruction::GameClose => game_close(program_id, accounts),
        }
    }
}

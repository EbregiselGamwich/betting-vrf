use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::pubkey::Pubkey;

use super::{game::BetInput, BettingAccount, StateAccountType};

#[derive(BorshSerialize, BorshDeserialize, Clone, ShankAccount)]
pub struct VrfResult {
    pub account_type: StateAccountType,
    pub is_fullfilled: bool,
    pub is_used: bool,
    pub marked_for_close: bool,
    pub owner: Pubkey,
    pub game: Pubkey,
    pub bet_id: u32,
    pub alpha: [u8; 72],
    pub beta: [u8; 64],
    pub pi: [u8; 80],
    pub locked_bettor_lamports: u64,
    pub locked_host_lamports: u64,
    pub bet_input: BetInput,
}

impl VrfResult {
    pub fn new(owner: Pubkey, game: Pubkey, bet_id: u32, alpha: [u8; 72], locked_bettor_lamports: u64, locked_host_lamports: u64, bet_input: BetInput) -> Self {
        Self {
            account_type: StateAccountType::Vrf,
            is_fullfilled: false,
            is_used: false,
            marked_for_close: false,
            owner,
            game,
            bet_id,
            alpha,
            beta: [0; 64],
            pi: [0; 80],
            locked_bettor_lamports,
            locked_host_lamports,
            bet_input,
        }
    }
}

impl BettingAccount for VrfResult {
    const ACCOUNT_TYPE: StateAccountType = StateAccountType::Vrf;
}

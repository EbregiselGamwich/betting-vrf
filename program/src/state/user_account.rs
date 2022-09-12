use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::pubkey::Pubkey;

use super::{BettingAccount, StateAccountType};

#[derive(BorshSerialize, BorshDeserialize, Clone, ShankAccount)]
pub struct UserAccount {
    pub account_type: StateAccountType,
    pub authority: Pubkey,
    pub total_bets: u32,
    pub current_lamports: u64,
    pub active_vrf_results: u32,
    pub games_hosted: u32,
    pub referral: Option<Pubkey>,
    pub username: Option<String>,
}

impl UserAccount {
    pub fn new(authority: Pubkey, referral: Option<Pubkey>, username: Option<String>) -> Self {
        Self {
            account_type: StateAccountType::UserAccount,
            authority,
            total_bets: 0,
            current_lamports: 0,
            active_vrf_results: 0,
            games_hosted: 0,
            referral,
            username,
        }
    }
}
impl BettingAccount for UserAccount {
    const ACCOUNT_TYPE: StateAccountType = StateAccountType::UserAccount;
}

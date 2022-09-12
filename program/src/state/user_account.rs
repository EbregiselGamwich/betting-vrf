use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::pubkey::Pubkey;

use crate::constants::PROFIT_SHARE;

use super::{BettingAccount, StateAccountType};

#[derive(BorshSerialize, BorshDeserialize, Clone, ShankAccount)]
pub struct UserAccount {
    pub account_type: StateAccountType,
    pub authority: Pubkey,
    pub total_bets: u32,
    pub current_lamports: u64,
    pub lamports_deposited: u64,
    pub lamports_withdrew: u64,
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
            lamports_deposited: 0,
            lamports_withdrew: 0,
            active_vrf_results: 0,
            games_hosted: 0,
            referral,
            username,
        }
    }
    pub fn get_profit_share(&self, withdraw_amount: u64) -> u64 {
        assert!(withdraw_amount <= self.current_lamports);
        let profit = if withdraw_amount + self.lamports_withdrew <= self.lamports_deposited {
            0
        } else if self.lamports_withdrew <= self.lamports_deposited && self.lamports_withdrew + withdraw_amount > self.lamports_deposited {
            self.lamports_withdrew + withdraw_amount - self.lamports_deposited
        } else {
            withdraw_amount
        };
        profit * PROFIT_SHARE / 10000
    }
}
impl BettingAccount for UserAccount {
    const ACCOUNT_TYPE: StateAccountType = StateAccountType::UserAccount;
}

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;

use super::{BettingAccount, StateAccountType};

#[derive(BorshSerialize, BorshDeserialize, Clone, ShankAccount)]
pub struct Stats {
    pub account_type: StateAccountType,
    pub total_games: u64,
    pub total_users: u32,
    pub total_bets: u32,
    pub total_wager: u64,
    pub total_lamports_won_by_bettors: u64,
}
impl Stats {
    pub fn new() -> Self {
        Self {
            account_type: StateAccountType::Stats,
            total_games: 0,
            total_bets: 0,
            total_wager: 0,
            total_lamports_won_by_bettors: 0,
            total_users: 0,
        }
    }
}
impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
impl BettingAccount for Stats {
    const ACCOUNT_TYPE: StateAccountType = StateAccountType::Stats;
}

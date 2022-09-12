pub mod mint_create;
pub mod stats_account_create;
pub mod user_account_create;

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;

use self::user_account_create::UserAccountCreateArgs;

#[derive(BorshSerialize, BorshDeserialize, ShankInstruction, Clone)]
pub enum BettingInstruction {
    #[account(0, writable, signer, name = "operator", desc = "Operator Account")]
    #[account(1, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(2, name = "system_program", desc = "System Program Account")]
    StatsAccountCreate,
    #[account(0, writable, signer, name = "operator", desc = "Operator Account")]
    #[account(1, writable, name = "mint_account", desc = "Mint Account")]
    #[account(2, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, name = "token_program", desc = "Token Program Account")]
    #[account(4, name = "system_program", desc = "System Program Account")]
    MintCreate,
    #[account(0, writable, signer, name = "user_wallet_account", desc = "User Wallet Account")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, name = "system_program", desc = "System Program Account")]
    UserAccountCreate { args: UserAccountCreateArgs },
}

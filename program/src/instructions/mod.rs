pub mod stats_account_create;
pub mod user_account_close;
pub mod user_account_create;
pub mod user_account_deposit;
pub mod user_account_withdraw;

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;

use self::{user_account_create::UserAccountCreateArgs, user_account_deposit::UserAccountDepositArgs, user_account_withdraw::UserAccountWithdrawArgs};

#[derive(BorshSerialize, BorshDeserialize, ShankInstruction, Clone)]
pub enum BettingInstruction {
    #[account(0, writable, signer, name = "operator", desc = "Operator Account")]
    #[account(1, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(2, name = "system_program", desc = "System Program Account")]
    StatsAccountCreate,
    #[account(0, writable, signer, name = "user_wallet_account", desc = "User Wallet Account")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, name = "system_program", desc = "System Program Account")]
    UserAccountCreate { args: UserAccountCreateArgs },
    #[account(0, writable, signer, name = "depositor", desc = "The account to transfer lamports from")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, name = "system_program", desc = "System Program Account")]
    UserAccountDeposit { args: UserAccountDepositArgs },
    #[account(0, writable, signer, name = "user_wallet_account", desc = "User Wallet Account")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, writable, name = "operator", desc = "Operator Account")]
    #[account(4, writable, optional, name = "referall_account", desc = "Referral Wallet Account")]
    UserAccountWithdraw { args: UserAccountWithdrawArgs },
    #[account(0, writable, signer, name = "user_wallet_account", desc = "User Wallet Account")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, name = "system_program", desc = "System Program Account")]
    UserAccountClose,
}

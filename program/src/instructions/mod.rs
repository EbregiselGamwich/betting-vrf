pub mod game_create;
pub mod game_place_bet;
pub mod game_set_active;
pub mod stats_account_create;
pub mod user_account_close;
pub mod user_account_create;
pub mod user_account_deposit;
pub mod user_account_withdraw;
pub mod vrf_result_close;
pub mod vrf_result_fullfill;
pub mod vrf_result_mark_close;

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;

use self::{
    game_create::GameCreateArgs, game_place_bet::GamePlaceBetArgs, game_set_active::GameSetActiveArgs, user_account_create::UserAccountCreateArgs,
    user_account_deposit::UserAccountDepositArgs, user_account_withdraw::UserAccountWithdrawArgs, vrf_result_fullfill::VrfResultFullfillArgs,
};

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
    #[account(0, writable, signer, name = "host", desc = "The wallet account of the host")]
    #[account(1, writable, name = "user_account", desc = "User Betting Account of the host")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, writable, name = "game_pda", desc = "Game PDA Account")]
    #[account(4, name = "system_program", desc = "System Program Account")]
    GameCreate { args: GameCreateArgs },
    #[account(0, signer, name = "host", desc = "The wallet account of the host")]
    #[account(1, writable, name = "game_pda", desc = "Game PDA Account")]
    GameSetActive { args: GameSetActiveArgs },
    #[account(0, writable, signer, name = "bettor", desc = "Bettor wallet account")]
    #[account(1, writable, name = "bettor_user_account", desc = "Bettor user account")]
    #[account(2, writable, name = "stats_pda", desc = "Stats PDA Account")]
    #[account(3, writable, name = "game_pda", desc = "Game PDA Account")]
    #[account(4, writable, name = "host_user_account", desc = "Host user account")]
    #[account(5, writable, name = "vrf_result_pda", desc = "VRF result PDA account")]
    #[account(6, name = "slot_hashes", desc = "Slot hashes account")]
    #[account(7, name = "system_program", desc = "System Program Account")]
    GamePlaceBet { args: GamePlaceBetArgs },
    #[account(0, signer, name = "operator", desc = "Operator Account")]
    #[account(1, writable, name = "vrf_result_pda", desc = "VRF result PDA account")]
    VrfResultFullfill { args: VrfResultFullfillArgs },
    #[account(0, signer, name = "bettor", desc = "Bettor wallet account")]
    #[account(1, writable, name = "vrf_result_pda", desc = "VRF result PDA account")]
    VrfResultMarkClose,
    #[account(0, writable, name = "vrf_result_pda", desc = "VRF result PDA account")]
    #[account(1, writable, signer, name = "bettor", desc = "Bettor wallet account")]
    #[account(2, writable, name = "bettor_user_account", desc = "Bettor user account")]
    #[account(3, name = "system_program", desc = "System Program Account")]
    VrfResultClose,
}

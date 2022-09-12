use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::error::BettingError;

pub fn check_is_writable(account_info: &AccountInfo) -> ProgramResult {
    if !account_info.is_writable {
        msg!("Expect account {} to be writable", account_info.key);
        Err(ProgramError::from(BettingError::AccountNotWritable))
    } else {
        Ok(())
    }
}
pub fn check_is_signer(account_info: &AccountInfo) -> ProgramResult {
    if !account_info.is_signer {
        msg!("Expect account {} to sign the transaction", account_info.key);
        Err(ProgramError::from(BettingError::AccountNotSigner))
    } else {
        Ok(())
    }
}
pub fn check_pda_cannonical_bump(account_info: &AccountInfo, seeds: &[&[u8]]) -> Result<u8, ProgramError> {
    let (pda, bump) = Pubkey::find_program_address(seeds, &crate::id());
    if account_info.key == &pda {
        Ok(bump)
    } else {
        msg!("Expect account {} to be PDA {}", account_info.key, &pda);
        Err(ProgramError::from(BettingError::WrongPDA))
    }
}
pub fn check_pubkey_eq(account_info: &AccountInfo, expected: &Pubkey) -> ProgramResult {
    if account_info.key == expected {
        Ok(())
    } else {
        msg!("Expect account {} to be {}", account_info.key, &expected);
        Err(ProgramError::from(BettingError::WrongPubkey))
    }
}

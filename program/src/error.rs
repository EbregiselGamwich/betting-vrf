use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Error, Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum BettingError {
    #[error("NoAuthority")]
    NoAuthority,
    #[error("AccountNotWritable")]
    AccountNotWritable,
    #[error("AccountNotSigner")]
    AccountNotSigner,
    #[error("WrongPDA")]
    WrongPDA,
    #[error("WrongPubkey")]
    WrongPubkey,
    #[error("WrongAccountOwner")]
    WrongAccountOwner,
    #[error("VrfResultNotFullfilled")]
    VrfResultNotFullfilled,
    #[error("VrfResultAlreadyUsed")]
    VrfResultAlreadyUsed,
    #[error("UserAccountNotSettled")]
    UserAccountNotSettled,
    #[error("GameNotActive")]
    GameNotActive,
    #[error("VrfResultAlreadyFullfilled")]
    VrfResultAlreadyFullfilled,
    #[error("VrfResultNotUsed")]
    VrfResultNotUsed,
    #[error("VrfResultNotMarkedForClose")]
    VrfResultNotMarkedForClose,
}
impl PrintProgramError for BettingError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + solana_program::decode_error::DecodeError<E> + PrintProgramError + num_traits::FromPrimitive,
    {
        msg!(&self.to_string());
    }
}
impl From<BettingError> for ProgramError {
    fn from(e: BettingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for BettingError {
    fn type_of() -> &'static str {
        "BettingError"
    }
}

pub use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Loan Already Taken")]
    LoanAlreadyTaken,
}

use anchor_lang::prelude::*;

#[account]
pub struct ActiveLoan {
    /// Collection
    pub collection: Pubkey,

    /// Offer Account
    pub offer_account: Pubkey,

    /// Lender
    pub lender: Pubkey,

    /// Borrower
    pub borrower: Pubkey,

    /// NFT Mint
    pub mint: Pubkey,

    /// Loan Taken Timestamp
    pub loan_ts: i64,

    /// Repayment Timestamp
    pub repay_ts: i64,

    /// Repaid
    pub is_repaid: bool,

    /// Liquidated
    pub is_liquidated: bool,

    /// Bump
    pub bump: u8,
}

impl ActiveLoan {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 1 + 1 + 1;
}

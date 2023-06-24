use anchor_lang::prelude::*;

declare_id!("DZSXK8Tvqo4vGqhW9mGjFuWX5XFcPGoJ5daJhMhxLFuK");

/// STATES
/// CollectionPool
/// DepositAccount
/// LoanAccount
/// INSTRUCTIONS
/// create_collection_pool
/// offer_loan
/// withdraw_offer
/// borrow
/// repay
/// liquidate

#[program]
pub mod nft_lend_borrow {
    use super::*;
}

#[account]
pub struct CollectionPool {
    /// NFT Collection ID
    pub collection_id: Pubkey,

    /// Switchboard Feed Aggregator
    pub switchboard_aggregator: Pubkey,

    /// Pool Owner
    pub pool_owner: Pubkey,

    /// Coefficient to calculate quote prices
    pub quote_coefficient: u64,

    /// Loan Duration
    pub duration: u64,
}

#[account]
pub struct Offer {
    /// Collection
    pub collection: Pubkey,

    /// Offer Amount
    pub offer_lamport_amount: u64,

    /// Loan Taken
    pub is_loan_taken: bool,
}

#[account]
pub struct ActiveLoan {
    /// Collection
    pub collection: Pubkey,

    /// Offer Account
    pub offer_account: Pubkey,

    /// NFT Mint
    pub mint: Pubkey,

    /// Repayment Timestamp
    pub repay_ts: Pubkey,

    /// Repaid
    pub is_repaid: bool,

    /// Liquidated
    pub is_liquidated: bool,
}

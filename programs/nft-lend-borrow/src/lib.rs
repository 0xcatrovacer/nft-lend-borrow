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

    pub fn create_pool(
        ctx: Context<CreatePool>,
        collection_id: Pubkey,
        duration: u64,
    ) -> Result<()> {
        let collection = &mut ctx.accounts.collection_pool;

        collection.collection_id = collection_id;
        collection.pool_owner = ctx.accounts.authority.key();
        collection.duration = duration;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct CreatePool<'info> {
    #[account(
        init,
        seeds=[b"collection_pool", collection_id.to_string().as_bytes()],
        bump,
        payer=authority,
        space=CollectionPool::LEN
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct CollectionPool {
    /// NFT Collection ID
    pub collection_id: Pubkey,

    /// Pool Owner
    pub pool_owner: Pubkey,

    /// Loan Duration
    pub duration: u64,
}

impl CollectionPool {
    pub const LEN: usize = 8 + 32 + 32 + 8;
}

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

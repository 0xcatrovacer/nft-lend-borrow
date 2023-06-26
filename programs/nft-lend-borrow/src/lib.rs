use anchor_lang::prelude::*;
use anchor_spl::token::{self, SetAuthority, Token, TokenAccount, Transfer};

declare_id!("DZSXK8Tvqo4vGqhW9mGjFuWX5XFcPGoJ5daJhMhxLFuK");

/// STATES
/// CollectionPool
/// DepositAccount
/// LoanAccount
///
/// INSTRUCTIONS
/// create_collection_pool
/// offer_loan
/// withdraw_offer
/// borrow
/// repay
/// liquidate

#[program]
pub mod nft_lend_borrow {
    use anchor_spl::token::spl_token::instruction::AuthorityType;

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
        collection.total_offers = 0;

        Ok(())
    }

    pub fn offer_loan(ctx: Context<OfferLoan>, offer_amount: u64) -> Result<()> {
        let offer_account = &mut ctx.accounts.offer_loan;
        let collection = &mut ctx.accounts.collection_pool;

        offer_account.collection = collection.key();
        offer_account.offer_lamport_amount = offer_amount;
        offer_account.lender = ctx.accounts.lender.key();

        collection.total_offers += 1;

        let (offer_account_authority, _offer_account_bump) = Pubkey::find_program_address(
            &[
                b"offer-token-account",
                collection.key().as_ref(),
                ctx.accounts.lender.key().as_ref(),
                collection.total_offers.to_string().as_bytes(),
            ],
            ctx.program_id,
        );

        token::set_authority(
            ctx.accounts.set_authority_context(),
            AuthorityType::AccountOwner,
            Some(offer_account_authority),
        )?;

        token::transfer(ctx.accounts.transfer_to_vault_context(), offer_amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct CreatePool<'info> {
    #[account(
        init,
        seeds=[b"collection_pool", collection_id.key().as_ref()],
        bump,
        payer=authority,
        space=CollectionPool::LEN
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OfferLoan<'info> {
    #[account(
        init,
        seeds=[b"offer", collection_pool.key().as_ref(), lender.key().as_ref(), collection_pool.total_offers.to_string().as_bytes()],
        bump,
        payer=lender,
        space=Offer::LEN
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        init,
        seeds = [b"offer-token-account", collection_pool.key().as_ref(), lender.key().as_ref(), collection_pool.total_offers.to_string().as_bytes()],
        bump,
        payer = lender,
        space=TokenAccount::LEN
    )]
    pub offer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lender_token_account.owner == *lender.key
    )]
    pub lender_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub lender: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

impl<'info> OfferLoan<'info> {
    fn transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.lender_token_account.to_account_info().clone(),
            to: self.offer_token_account.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.offer_token_account.to_account_info().clone(),
            current_authority: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[account]
pub struct CollectionPool {
    /// NFT Collection ID
    pub collection_id: Pubkey,

    /// Pool Owner
    pub pool_owner: Pubkey,

    /// Loan Duration
    pub duration: u64,

    /// Total Loans
    pub total_offers: u64,
}

impl CollectionPool {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8;
}

#[account]
pub struct Offer {
    /// Collection
    pub collection: Pubkey,

    /// Offer Amount
    pub offer_lamport_amount: u64,

    /// Lender
    pub lender: Pubkey,

    /// Loan Taken
    pub is_loan_taken: bool,

    /// Borrower
    pub borrower: Pubkey,
}

impl Offer {
    pub const LEN: usize = 8 + 32 + 8 + 1 + 32;
}

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

    /// Repayment Timestamp
    pub repay_ts: Pubkey,

    /// Repaid
    pub is_repaid: bool,

    /// Liquidated
    pub is_liquidated: bool,
}

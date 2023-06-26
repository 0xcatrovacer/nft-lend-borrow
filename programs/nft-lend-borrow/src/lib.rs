use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, spl_token::instruction::AuthorityType, CloseAccount, SetAuthority, Token, TokenAccount,
    Transfer,
};

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

    pub fn offer_loan(
        ctx: Context<OfferLoan>,
        offer_amount: u64,
        _collection_id: Pubkey,
    ) -> Result<()> {
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

    pub fn withdraw_offer(ctx: Context<WithdrawOffer>, _collection_id: Pubkey) -> Result<()> {
        let collection = &mut ctx.accounts.collection_pool;

        if ctx.accounts.offer_loan.is_loan_taken == true {
            return Err(ErrorCode::LoanAlreadyTaken.into());
        }

        collection.total_offers -= 1;

        let (_token_account_authority, token_account_bump) = Pubkey::find_program_address(
            &[
                b"offer-token-account",
                collection.key().as_ref(),
                ctx.accounts.lender.key().as_ref(),
                collection.total_offers.to_string().as_bytes(),
            ],
            ctx.program_id,
        );

        let key = collection.key();
        let lender = ctx.accounts.lender.key();
        let offer_bytes = collection.total_offers.to_string();

        let collection_key: &[u8] = key.as_ref().try_into().expect("");
        let lender_key: &[u8] = lender.as_ref().try_into().expect("");
        let total_offers_bytes: &[u8] = offer_bytes.as_bytes().try_into().expect("");

        let authority_seeds_1: &[&[u8]] = &[
            b"offer-token-account",
            collection_key,
            lender_key,
            total_offers_bytes,
        ];

        let authority_seeds_2: &[&[u8]] = &[&[token_account_bump]];

        let authority_seeds = &[authority_seeds_1, authority_seeds_2];

        token::transfer(
            ctx.accounts
                .transfer_to_lender_context()
                .with_signer(&authority_seeds[..]),
            ctx.accounts.offer_token_account.amount,
        )?;

        token::close_account(
            ctx.accounts
                .close_account_context()
                .with_signer(&authority_seeds[..]),
        )?;

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
#[instruction(collection_id: Pubkey)]
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

    #[account(
        mut,
        seeds=[b"collection_pool", collection_id.key().as_ref()],
        bump=collection_pool.bump
    )]
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

        CpiContext::new(self.system_program.to_account_info().clone(), cpi_accounts)
    }

    fn set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.offer_token_account.to_account_info().clone(),
            current_authority: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct WithdrawOffer<'info> {
    #[account(
        mut,
        seeds=[b"offer", collection_pool.key().as_ref(), lender.key().as_ref(), collection_pool.total_offers.to_string().as_bytes()],
        bump=offer_loan.bump,
        close=lender,
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        mut,
        seeds = [b"offer-token-account", collection_pool.key().as_ref(), lender.key().as_ref(), collection_pool.total_offers.to_string().as_bytes()],
        bump=offer_loan.bump,
    )]
    pub offer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[b"collection_pool", collection_id.key().as_ref()],
        bump=collection_pool.bump
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub lender: Signer<'info>,

    #[account(
        mut,
        constraint = lender_token_account.owner == *lender.key
    )]
    pub lender_token_account: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous
    pub token_account_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawOffer<'info> {
    fn transfer_to_lender_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.offer_token_account.to_account_info().clone(),
            to: self.lender_token_account.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn close_account_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.offer_token_account.to_account_info().clone(),
            destination: self.lender.to_account_info().clone(),
            authority: self.token_account_authority.clone(),
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

    /// Bump
    pub bump: u8,
}

impl CollectionPool {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 1;
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

    /// Bump
    pub bump: u8,
}

impl Offer {
    pub const LEN: usize = 8 + 32 + 8 + 1 + 32 + 1;
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

#[error_code]
pub enum ErrorCode {
    #[msg("Loan Already Taken")]
    LoanAlreadyTaken,
}

pub use anchor_lang::prelude::*;

use anchor_lang::system_program;
use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint, SetAuthority};

use crate::states::{ActiveLoan, Offer, CollectionPool};

use crate::errors::ErrorCode;

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct Borrow<'info> {
    #[account(
        init,
        seeds = [b"active-loan", offer_loan.key().as_ref()],
        bump,
        payer = borrower,
        space = ActiveLoan::LEN
    )]
    pub active_loan: Box<Account<'info, ActiveLoan>>,

    #[account(
        mut,
        seeds = [
            b"offer", 
            collection_pool.key().as_ref(), 
            offer_loan.lender.key().as_ref(), 
            collection_pool.total_offers.to_string().as_bytes()
        ],
        bump = offer_loan.bump
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        mut,
        seeds = [
            b"vault-token-account", 
            offer_loan.key().as_ref(),
        ],
        bump = offer_loan.bump,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        seeds = [
            b"offer-asset-account",
            offer_loan.key().as_ref(),
        ],
        bump,
        payer = borrower,
        token::mint = asset_mint,
        token::authority = borrower
    )]
    pub vault_asset_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[b"collection_pool", collection_id.key().as_ref()],
        bump=collection_pool.bump
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(
        mut,
        constraint = borrower_asset_account.owner == *borrower.key,
        constraint = borrower_asset_account.mint == *asset_mint.to_account_info().key
    )]
    pub borrower_asset_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub asset_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Borrow<'info> {
    fn transfer_to_borrower_context(&self) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let cpi_accounts = system_program::Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.borrower.to_account_info().clone(),
        };

        CpiContext::new(self.system_program.to_account_info().clone(), cpi_accounts)
    }

    fn transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.borrower_asset_account.to_account_info().clone(),
            to: self.vault_asset_account.to_account_info().clone(),
            authority: self.borrower.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    fn set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_asset_account.to_account_info().clone(),
            current_authority: self.borrower.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}


    pub fn handler(
        ctx: Context<Borrow>,
        minimum_balance_for_rent_exemption: u64,
        _collection_id: Pubkey,
    ) -> Result<()> {
        let active_loan = &mut ctx.accounts.active_loan;
        let offer = &mut ctx.accounts.offer_loan;
        let collection = &mut ctx.accounts.collection_pool;

        if offer.is_loan_taken == true {
            return Err(ErrorCode::LoanAlreadyTaken.into());
        }

        active_loan.collection = collection.key();
        active_loan.offer_account = offer.key();
        active_loan.lender = offer.lender.key();
        active_loan.borrower = ctx.accounts.borrower.key();
        active_loan.mint = ctx.accounts.asset_mint.key();
        active_loan.loan_ts = ctx.accounts.clock.unix_timestamp;
        active_loan.repay_ts = ctx.accounts.clock.unix_timestamp + collection.duration;
        active_loan.is_repaid = false;
        active_loan.is_liquidated = false;
        active_loan.bump = *ctx.bumps.get("active_loan").unwrap();

        offer.borrower = ctx.accounts.borrower.key();
        offer.is_loan_taken = true;

        let (_token_account_authority, token_account_bump) = Pubkey::find_program_address(
            &[
                b"vault-token-account",
                collection.key().as_ref(),
                offer.lender.key().as_ref(),
                collection.total_offers.to_string().as_bytes(),
            ],
            ctx.program_id,
        );

        let key = collection.key();
        let lender = offer.lender.key();
        let offer_bytes = collection.total_offers.to_string();

        let collection_key: &[u8] = key.as_ref().try_into().expect("");
        let lender_key: &[u8] = lender.as_ref().try_into().expect("");
        let total_offers_bytes: &[u8] = offer_bytes.as_bytes().try_into().expect("");

        let authority_seeds_1: &[&[u8]] = &[
            b"vault-token-account",
            collection_key,
            lender_key,
            total_offers_bytes,
        ];

        let authority_seeds_2: &[&[u8]] = &[&[token_account_bump]];

        let authority_seeds = &[authority_seeds_1, authority_seeds_2];

        let vault_lamports_initial = ctx
            .accounts
            .vault_token_account
            .to_account_info()
            .lamports();
        let transfer_amount = vault_lamports_initial
            .checked_sub(minimum_balance_for_rent_exemption)
            .unwrap();

        let (vault_account_authority, _vault_account_bump) = Pubkey::find_program_address(
            &[
                b"vault-token-account",
                collection.key().as_ref(),
                offer.lender.key().as_ref(),
                collection.total_offers.to_string().as_bytes(),
            ],
            ctx.program_id,
        );

        token::transfer(ctx.accounts.transfer_to_vault_context(), 1)?;

        system_program::transfer(
            ctx.accounts
                .transfer_to_borrower_context()
                .with_signer(&authority_seeds[..]),
            transfer_amount,
        )?;

        token::set_authority(
            ctx.accounts.set_authority_context(),
            AuthorityType::AccountOwner,
            Some(vault_account_authority),
        )?;

        Ok(())
    }
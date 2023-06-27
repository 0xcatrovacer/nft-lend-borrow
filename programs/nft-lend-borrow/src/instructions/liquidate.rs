pub use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Mint, Token, Transfer};

use crate::errors::ErrorCode;
pub use crate::states::{Offer, ActiveLoan, CollectionPool};

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct Liquidate<'info> {
    #[account(
        mut,
        seeds=[b"active-loan", offer.key().as_ref()],
        bump=active_loan.bump
    )]
    pub active_loan: Box<Account<'info, ActiveLoan>>,

    #[account(
        mut,
        seeds=[
            b"offer", 
            collection_pool.key().as_ref(), 
            offer.lender.key().as_ref(), 
            collection_pool.total_offers.to_string().as_bytes()
        ],
        bump=offer.bump
    )]
    pub offer: Box<Account<'info, Offer>>,

    #[account(
        mut,
        seeds=[b"collection_pool", collection_id.key().as_ref()],
        bump=collection_pool.bump
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(
        mut,
        constraint = lender_token_account.owner == offer.lender.key()
    )]
    pub lender_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub asset_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = vault_asset_account.mint == asset_mint.key(),
        constraint = vault_asset_account.owner == asset_account_authority.key()
    )]
    pub vault_asset_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lender_asset_account.mint == asset_mint.key(),
        constraint = lender_asset_account.owner == lender.key()
    )]
    pub lender_asset_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub lender: Signer<'info>,

    /// CHECK: This is not dangerous
    pub asset_account_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Liquidate<'info> {
    fn transfer_asset_to_lender_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_asset_account.to_account_info().clone(),
            to: self.lender_asset_account.to_account_info().clone(),
            authority: self.asset_account_authority.clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[access_control(repayment_time_over(&ctx.accounts.active_loan, &ctx.accounts.clock))]
pub fn handler(ctx: Context<Liquidate>, _collection_id: Pubkey) -> Result<()> {
    let active_loan = &mut ctx.accounts.active_loan;
    let offer = &mut ctx.accounts.offer;
    let collection = &mut ctx.accounts.collection_pool;

    if active_loan.is_repaid {
        return Err(ErrorCode::LoanAlreadyRepaid.into())
    }

    active_loan.is_liquidated = true;

    let (_token_account_authority, token_account_bump) = Pubkey::find_program_address(
        &[
        b"offer-token-account",
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
    b"offer-token-account",
    collection_key,
    lender_key,
    total_offers_bytes,
    ];
    let authority_seeds_2: &[&[u8]] = &[&[token_account_bump]];

    let authority_seeds = &[authority_seeds_1, authority_seeds_2];

    token::transfer(ctx.accounts.transfer_asset_to_lender_context().with_signer(&authority_seeds[..]), 1)?;

    Ok(())
}

// Access Control Modifier
fn repayment_time_over<'info>(active_loan: &Account<'info, ActiveLoan>, clock: &Sysvar<'info, Clock>) -> Result<()> {
    if !(active_loan.repay_ts < clock.unix_timestamp) {
        return Err(ErrorCode::CannotLiquidateYet.into())
    }

    Ok(())
}
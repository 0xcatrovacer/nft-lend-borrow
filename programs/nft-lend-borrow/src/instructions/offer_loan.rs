pub use anchor_lang::prelude::*;

use anchor_lang::system_program;
use anchor_spl::token::{
    self, spl_token::instruction::AuthorityType, SetAuthority, Token, TokenAccount,
};

pub use crate::states::{CollectionPool, Offer};

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct OfferLoan<'info> {
    #[account(
        init,
        seeds=[
            b"offer",
            collection_pool.key().as_ref(),
            lender.key().as_ref(),
            collection_pool.total_offers.to_string().as_bytes(),
        ],
        bump,
        payer=lender,
        space=Offer::LEN
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        init,
        seeds = [
            b"offer-token-account", 
            offer_loan.key().as_ref()
        ],
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
    fn transfer_to_vault_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let cpi_accounts = system_program::Transfer {
            from: self.lender.to_account_info().clone(),
            to: self.offer_token_account.to_account_info().clone(),
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

pub fn handler(ctx: Context<OfferLoan>, offer_amount: u64, _collection_id: Pubkey) -> Result<()> {
    let offer_account = &mut ctx.accounts.offer_loan;
    let collection = &mut ctx.accounts.collection_pool;

    offer_account.collection = collection.key();
    offer_account.offer_lamport_amount = offer_amount;
    offer_account.repay_lamport_amount = offer_amount + offer_amount * 10 / 100;
    offer_account.lender = ctx.accounts.lender.key();
    offer_account.bump = *ctx.bumps.get("offer_loan").unwrap();

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

    system_program::transfer(ctx.accounts.transfer_to_vault_context(), offer_amount)?;

    Ok(())
}

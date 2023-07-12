pub use anchor_lang::prelude::*;

use anchor_lang::system_program;

use crate::states::{CollectionPool, Offer, Vault};

use crate::errors::ErrorCode;

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct WithdrawOffer<'info> {
    #[account(
        mut,
        close = lender,
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        mut,
        close = lender
    )]
    pub vault_account: Account<'info, Vault>,

    #[account(
        mut,
        seeds = [b"collection_pool", collection_id.key().as_ref()],
        bump = collection_pool.bump
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub lender: Signer<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawOffer<'info> {
    fn transfer_to_lender_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let cpi_accounts = system_program::Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.system_program.to_account_info().clone(), cpi_accounts)
    }
}

pub fn handler(
    ctx: Context<WithdrawOffer>,
    minimum_balance_for_rent_exemption: u64,
    _collection_id: Pubkey,
) -> Result<()> {
    let collection = &mut ctx.accounts.collection_pool;

    if ctx.accounts.offer_loan.is_loan_taken == true {
        return Err(ErrorCode::LoanAlreadyTaken.into());
    }

    collection.total_offers -= 1;

    let vault_lamports_initial = ctx.accounts.vault_account.to_account_info().lamports();
    let transfer_amount = vault_lamports_initial
        .checked_sub(minimum_balance_for_rent_exemption)
        .unwrap();

    system_program::transfer(ctx.accounts.transfer_to_lender_context(), transfer_amount)?;

    Ok(())
}

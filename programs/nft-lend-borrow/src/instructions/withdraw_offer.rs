pub use anchor_lang::prelude::*;

use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::states::{CollectionPool, Offer};

use crate::errors::ErrorCode;

#[derive(Accounts)]
#[instruction(collection_id: Pubkey)]
pub struct WithdrawOffer<'info> {
    #[account(
        mut,
        seeds=[
            b"offer",
            collection_pool.key().as_ref(),
            lender.key().as_ref(),
            collection_pool.total_offers.to_string().as_bytes()
        ],
        bump = offer_loan.bump,
        close = lender,
    )]
    pub offer_loan: Box<Account<'info, Offer>>,

    #[account(
        mut,
        seeds = [
            b"vault-token-account", 
            offer_loan.key().as_ref()
        ],
        bump = offer_loan.bump,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"collection_pool", collection_id.key().as_ref()],
        bump = collection_pool.bump
    )]
    pub collection_pool: Box<Account<'info, CollectionPool>>,

    #[account(mut)]
    pub lender: Signer<'info>,

    /// CHECK: This is not dangerous
    pub token_account_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawOffer<'info> {
    fn transfer_to_lender_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.lender.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn close_account_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_token_account.to_account_info().clone(),
            destination: self.lender.to_account_info().clone(),
            authority: self.token_account_authority.clone(),
        };

        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
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

    let (_token_account_authority, token_account_bump) = Pubkey::find_program_address(
        &[
            b"vault-token-account",
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

    token::transfer(
        ctx.accounts
            .transfer_to_lender_context()
            .with_signer(&authority_seeds[..]),
        transfer_amount,
    )?;

    token::close_account(
        ctx.accounts
            .close_account_context()
            .with_signer(&authority_seeds[..]),
    )?;

    Ok(())
}

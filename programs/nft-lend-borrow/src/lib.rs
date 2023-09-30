pub mod errors;
pub mod instructions;
pub mod states;

pub use errors::ErrorCodes;
pub use instructions::*;
pub use states::*;

declare_id!("DZSXK8Tvqo4vGqhW9mGjFuWX5XFcPGoJ5daJhMhxLFuK");

#[program]
pub mod nft_lend_borrow {
    use super::*;

    pub fn create_pool(
        ctx: Context<CreatePool>,
        collection_id: Pubkey,
        duration: i64,
    ) -> Result<()> {
        instructions::create_pool::handler(ctx, collection_id, duration)
    }

    pub fn offer_loan(ctx: Context<OfferLoan>, offer_amount: u64) -> Result<()> {
        instructions::offer_loan::handler(ctx, offer_amount)
    }

    pub fn withdraw_offer(
        ctx: Context<WithdrawOffer>,
        minimum_balance_for_rent_exemption: u64,
    ) -> Result<()> {
        instructions::withdraw_offer::handler(
            ctx,
            minimum_balance_for_rent_exemption,
        )
    }

    pub fn borrow(ctx: Context<Borrow>, minimum_balance_for_rent_exemption: u64) -> Result<()> {
        instructions::borrow::handler(ctx, minimum_balance_for_rent_exemption)
    }

    pub fn repay(ctx: Context<Repay>) -> Result<()> {
        instructions::repay::handler(ctx)
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::handler(ctx)
    }
}

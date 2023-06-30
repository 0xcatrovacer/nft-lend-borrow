use anchor_lang::prelude::*;

#[account]
pub struct Vault {
    /// The offer this vault is linked to
    pub offer: Pubkey,

    /// Bump
    pub bump: u8,
}

impl Vault {
    pub const LEN: usize = 8 + 32 + 1;
}

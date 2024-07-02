use anchor_lang::prelude::*;

declare_id!("6YtXjMCqMFA4y2J7YAMGWm85Ae9R1G1at47k6kXKARuM");

#[program]
pub mod pump {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, mint_to, transfer, MintTo, Transfer};

declare_id!("6YtXjMCqMFA4y2J7YAMGWm85Ae9R1G1at47k6kXKARuM");

const AMOUNT: u64 = 1_000_000_000_000;
const INITIAL_LAMPORTS_PER_TOKEN: u64 = 1;
#[program]
pub mod pump {
    use anchor_spl::token::{set_authority, spl_token::instruction::AuthorityType, SetAuthority};

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        Ok(())
    }
    pub fn create(ctx: Context<Create>) -> Result<()> {
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.pool_token_holder_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            AMOUNT
        )?;
        set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: ctx.accounts.program_authority.to_account_info(),
                    account_or_mint: ctx.accounts.mint.to_account_info(),
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            AuthorityType::MintTokens,
            None
        )?;
        ctx.accounts.pool_account.token = AMOUNT;
        ctx.accounts.pool_account.sol = INITIAL_LAMPORTS_PER_TOKEN;
        Ok(())
    }
    pub fn buy(ctx: Context<Buy>, amount: u64, price: u64, slippage: u64) -> Result<()> {
        // x * y = k
        let k = ctx.accounts.pool_account.sol * ctx.accounts.pool_account.token;
        ctx.accounts.pool_account.sol += amount;
        let token_amount = k / ctx.accounts.pool_account.sol;
        let token_sent = ctx.accounts.pool_account.token - token_amount;
        ctx.accounts.pool_account.token = token_amount;
        let new_price = ctx.accounts.pool_account.sol.checked_mul(100).ok_or(CustomError::OverflowError)?.checked_div(ctx.accounts.pool_account.token.checked_mul(100).ok_or(CustomError::OverflowError)?).ok_or(CustomError::OverflowError)?;
        if new_price > price * (100 + slippage) || new_price < price * (100 - slippage) {
            return Err(CustomError::SlippageExceeded.into())
        }   
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_holder_account.to_account_info(),
                    to: ctx.accounts.signer_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            token_sent,
        )?;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.pool_sol_holder_account.to_account_info(),
                }
            ),
            amount
        )?;
        Ok(())
    }
    pub fn sell(ctx: Context<Sell>, amount: u64, price: u64, slippage: u64) -> Result<()> {
        let k = ctx.accounts.pool_account.sol * ctx.accounts.pool_account.token;
        ctx.accounts.pool_account.token += amount;
        let sol_amount = k / ctx.accounts.pool_account.token;
        let sol_sent = ctx.accounts.pool_account.sol - sol_amount;
        let new_price = ctx.accounts.pool_account.sol.checked_mul(100).ok_or(CustomError::OverflowError)?.checked_div(ctx.accounts.pool_account.token.checked_mul(100).ok_or(CustomError::OverflowError)?).ok_or(CustomError::OverflowError)?;
        if new_price > price * (100 + slippage) || new_price < price * (100 - slippage) {
            return Err(CustomError::SlippageExceeded.into())
        }   
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_token_account.to_account_info(),
                    to: ctx.accounts.pool_token_holder_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            amount
        )?;
        **ctx.accounts.pool_sol_holder_account.try_borrow_mut_lamports()? -= sol_sent;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += sol_sent;
        Ok(())
    }
}

#[error_code]
pub enum CustomError {
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Overflow Error")]
    OverflowError
}
#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    pub signer: Signer<'info>,
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"fee"],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub fee_account: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"auth"],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>
}

#[account]
pub struct Pool {
    pub sol: u64,
    pub token: u64,
}
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::authority = program_authority,
        mint::decimals = 6,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        seeds = [b"pool", mint.key().as_ref()],
        bump,
        space = 8 + 8 + 8
    )]
    pub pool_account: Account<'info, Pool>,
    #[account(
        init,
        payer = signer,
        seeds = [b"token_account", pool_account.key().as_ref()],
        bump,
        token::authority = program_authority,
        token::mint = mint,
    )]
    pub pool_token_holder_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = signer,
        seeds = [b"sol_account", pool_account.key().as_ref()],
        bump,
        space = 8,
    )]
    /// CHECK: 
    pub pool_sol_holder_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub pool_account: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"token_account", pool_account.key().as_ref()],
        bump,
    )]
    pub pool_token_holder_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol_account", pool_account.key().as_ref()],
        bump,
    )]
    /// CHECK:
    pub pool_sol_holder_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK:
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub pool_account: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"token_account", pool_account.key().as_ref()],
        bump,
    )]
    pub pool_token_holder_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol_account", pool_account.key().as_ref()],
        bump,
    )]
    /// CHECK:
    pub pool_sol_holder_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK:
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}


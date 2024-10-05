use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3};
use anchor_spl::token::{Mint, Token, TokenAccount};
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::types::DataV2;

use crate::state::Caller;

#[derive(Accounts, AnchorDeserialize)]
#[instruction(token_name: String, token_symbol: String, uri: String)]
pub struct CreateCaller<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(
        init,
        payer = caller,
        space = Caller::INIT_SPACE,
        seeds = [b"caller", caller.key().as_ref()],
        bump,
    )]
    pub caller_account: Account<'info, Caller>,
    #[account(
        init,
        seeds = [b"mint", caller.key().as_ref()],
        bump,
        payer = caller,
        mint::decimals = 8,
        mint::authority = mint.key(),
    )]
    pub mint: Account<'info, Mint>,
    ///CHECK: Using "address" constraint to validate metadata account address
    #[account(
        mut,
        address=Metadata::find_pda(&mint.key()).0
    )]
    pub metadata_account: UncheckedAccount<'info>,
    #[account(
        init,
        payer = caller,
        associated_token::mint = mint,
        associated_token::authority = mint,
    )]
    pub mint_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol_vault", caller.key().as_ref()],
        bump,
    )]
    pub sol_vault: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_metadata_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> CreateCaller<'info> {
    pub fn create_caller(
        ctx: Context<CreateCaller>,
        token_name: String,
        token_symbol: String,
        uri: String,
    ) -> Result<()> {
        let mint_supply = 1_000_000_000;
        let mint_total_supply = 1_000_000_000;
        let value_target = 1_000_000_000;

        msg!("Test");

        // On-chain token metadata for the mint
        let data_v2 = DataV2 {
            name: token_name.to_string(),
            symbol: token_symbol.to_string(),
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        // CPI Context
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(), // the metadata account being created
                mint: ctx.accounts.mint.to_account_info(), // the mint account of the metadata account
                mint_authority: ctx.accounts.mint.to_account_info(), // the mint authority of the mint account
                update_authority: ctx.accounts.mint.to_account_info(), // the update authority of the metadata account
                payer: ctx.accounts.caller.to_account_info(), // the payer for creating the metadata account
                system_program: ctx.accounts.system_program.to_account_info(), // the system program account
                rent: ctx.accounts.rent.to_account_info(), // the rent sysvar account
            },
        );

        create_metadata_accounts_v3(
            cpi_ctx, // cpi context
            data_v2, // token metadata
            true,    // is_mutable
            true,    // update_authority_is_signer
            None,    // collection details
        )?;

        // Create the mint account
        anchor_spl::token::initialize_mint(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::InitializeMint {
                    mint: ctx.accounts.mint.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            8,
            &ctx.accounts.mint.key(),
            Some(&ctx.accounts.mint.key()),
        )?;

        // Create the associated token account for the mint vault
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.caller.to_account_info(),
                associated_token: ctx.accounts.mint_vault.to_account_info(),
                authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ))?;

        // Create the associated token account for the mint vault
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.caller.to_account_info(),
                associated_token: ctx.accounts.mint_vault.to_account_info(),
                authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ))?;

        // Mint tokens to the mint vault
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.mint_vault.to_account_info(),
                    authority: ctx.accounts.mint.to_account_info(),
                },
                &[&[
                    b"mint",
                    ctx.accounts.caller.key().as_ref(),
                    &[ctx.bumps.mint],
                ]],
            ),
            mint_supply,
        )?;

        ctx.accounts.caller_account.set_inner(Caller {
            caller: ctx.accounts.caller.key(),
            mint: ctx.accounts.mint.key(), // Add this line
            mint_supply,
            mint_total_supply,
            value_target,
            mint_vault: ctx.accounts.mint_vault.key(),
            sol_vault_bump: ctx.bumps.sol_vault,
            bump: ctx.bumps.caller_account,
        });
        Ok(())
    }
}

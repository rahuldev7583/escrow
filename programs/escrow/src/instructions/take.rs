use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, CloseAccount},
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::Escrow;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Take<'info> {
    #[account(mut)]
    taker: Signer<'info>,

    #[account(mut)]
    maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(init_if_needed, payer = taker, associated_token::mint = mint_a, associated_token::authority = taker, associated_token::token_program= token_program)]
    pub taker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_b, associated_token::authority = taker, associated_token::token_program= token_program)]
    pub taker_ata_b: InterfaceAccount<'info, TokenAccount>,

    #[account(init_if_needed, payer = taker, associated_token::mint = mint_b, associated_token::authority = maker, associated_token::token_program= token_program)]
    pub maker_ata_b: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, close = maker, has_one = maker, has_one = mint_b, seeds = [b"escrow", seed.to_le_bytes().as_ref()],  bump)]
    pub escrow: Account<'info, Escrow>,

    #[account(mut, associated_token::mint = mint_a, associated_token::authority = escrow, associated_token::token_program= token_program)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    pub fn take(&mut self) -> Result<()> {
        let seeds = &[
            b"escrow",
            &self.escrow.seeds.to_le_bytes()[..],
            &[self.escrow.bumps],
        ];

        let signer_seeds = &[&seeds[..]];

        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let transfer_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        );

        transfer_checked(transfer_cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_accounts,
            signer_seeds,
        );
        close_account(close_cpi_ctx)?;

        let transfer_accounts_token_b_from_taker = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let transfer_cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            transfer_accounts_token_b_from_taker,
        );

        transfer_checked(
            transfer_cpi_ctx,
            self.escrow.receive_amount,
            self.mint_b.decimals,
        )?;

        Ok(())
    }
}

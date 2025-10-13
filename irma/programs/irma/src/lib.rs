// In programs/irma/src/lib.rs

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use std::mem::size_of;

// Import the state structs from your modules, as they are used in the account definitions.
use pricing::{StateMap, StableState};
use orca_integration::OrcaPoolState;

// Declare your program's ID
declare_id!("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");

// ====================================================================
// START: DEFINE ALL INSTRUCTION ACCOUNT STRUCTS HERE
// ====================================================================

#[derive(Accounts)]
pub struct Init<'info> {
    // Note: We need to qualify MAX_BACKING_COUNT with its module
    #[account(init, space=32 + 8 + size_of::<StableState>()*pricing::MAX_BACKING_COUNT, payer=irma_admin, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, StateMap>,
    #[account(mut)]
    pub irma_admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Common<'info> {
    #[account(mut)]
    pub state: Account<'info, StateMap>,
    pub trader: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Maint<'info> {
    #[account(mut)]
    pub state: Account<'info, StateMap>,
    pub irma_admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateOrcaPool<'info> {
    #[account(init, payer = admin, space = 8 + 256)]
    pub pool_state: Account<'info, OrcaPoolState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePoolState<'info> {
    #[account(mut)]
    pub pool_state: Account<'info, OrcaPoolState>,
    #[account(mut)]
    pub updater: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetPoolInfo<'info> {
    pub pool_state: Account<'info, OrcaPoolState>,
}

#[derive(Accounts)]
pub struct SimulateSwap<'info> {
    pub pool_state: Account<'info, OrcaPoolState>,
    #[account(mut)]
    pub trader: Signer<'info>,
}

// ====================================================================
// END: ACCOUNT STRUCT DEFINITIONS
// ====================================================================

// Declare your modules
// pub mod iopenbook;
pub mod orca_integration;
pub mod pricing;

#[program]
pub mod irma {
    use super::*; // This will now correctly bring Init, Maint, Common, etc. into scope

    pub fn initialize(ctx: Context<Init>) -> Result<()> {
        pricing::init_pricing(ctx)
    }

    pub fn add_reserve(ctx: Context<Maint>, symbol: String, mint_address: Pubkey, decimals: u8) -> Result<()> {
        msg!("Add stablecoin entry, size of StateMap: {}", size_of::<StateMap>());
        pricing::add_reserve(ctx, &symbol, mint_address, decimals)
    }

    pub fn remove_reserve(ctx: Context<Maint>, symbol: String) -> Result<()> {
        pricing::remove_reserve(ctx, &symbol)
    }

    pub fn disable_reserve(ctx: Context<Maint>, symbol: String) -> Result<()> {
        pricing::disable_reserve(ctx, &symbol)
    }

    pub fn update_mint_price_with_inflation(ctx: Context<Common>, quote_token: String, inflation_rate: f64) -> Result<()> {
        pricing::update_mint_price_with_inflation(ctx, &quote_token, inflation_rate)
    }

    pub fn get_redemption_price(ctx: Context<Common>, quote_token: String) -> Result<f64> {
        pricing::get_redemption_price(ctx, &quote_token)
    }

    pub fn get_prices(ctx: Context<Common>, quote_token: String) -> Result<(f64, f64)> {
        pricing::get_prices(ctx, &quote_token)
    }

    // Orca Integration Functions
    pub fn create_orca_pool(
        ctx: Context<CreateOrcaPool>,
        pool_id: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        fee_rate: u64,
        tick_spacing: u16,
    ) -> Result<()> {
        orca_integration::create_orca_pool(ctx, pool_id, token_a_mint, token_b_mint, fee_rate, tick_spacing)
    }

    pub fn update_pool_state(
        ctx: Context<UpdatePoolState>,
        current_price: u64,
        liquidity: u64,
        volume_24h: u64,
    ) -> Result<()> {
        orca_integration::update_pool_state(ctx, current_price, liquidity, volume_24h)
    }

    pub fn get_pool_info(ctx: Context<GetPoolInfo>) -> Result<orca_integration::OrcaPoolState> {
        orca_integration::get_pool_info(ctx)
    }

    pub fn simulate_swap(
        ctx: Context<SimulateSwap>,
        amount_in: u64,
        token_in_mint: Pubkey,
        min_amount_out: u64,
    ) -> Result<u64> {
        orca_integration::simulate_swap(ctx, amount_in, token_in_mint, min_amount_out)
    }
}
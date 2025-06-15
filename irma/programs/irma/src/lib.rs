#![allow(unexpected_cfgs)]
// #![feature(trivial_bounds)]
// use std::cmp::{
//     PartialEq,
//     Eq,
// };
// use bytemuck::{
//     Pod,
// };
// use solana_sdk_ids::system_program;
// use solana_program::{pubkey, pubkey::Pubkey};
use bytemuck::bytes_of_mut;
use anchor_lang::prelude::*;
// use anchor_lang::{declare_id, program};
use anchor_lang::{
    account,
    Accounts,
    // AnchorSerialize, 
    // AnchorDeserialize, 
    // declare_id,
    // program,
    Result,
    // ToAccountMetas, 
    // ToAccountInfos,
    zero_copy
};


// use anchor_lang::ZeroCopy;
// use crate::iopenbook::*;

// Dummy CPI context and consume_given_events for demonstration
// use anchor_lang::prelude::{AccountInfo, CpiContext, Signer, AccountLoader, Program, Pubkey, AnchorDeserialize, AnchorSerialize};
pub const IRMA_ID: Pubkey = pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");

pub mod iopenbook;
pub mod pricing;

use crate::pricing::Stablecoins;
use crate::pricing::{ /* Initialize, */ IrmaCommon};
use crate::pricing::{ /* initialize, */ set_mint_price, mint_irma, redeem_irma};
use crate::iopenbook::{EventHeap, Market, ConsumeEvents, EventHeapHeader, EventNode, AnyEvent, OracleConfig};
use crate::iopenbook::{/*OpenBookV2,*/ get_latest_slot, consume_given_events, MAX_NUM_EVENTS};


pub fn crank<'info>(ctx: &Context<'_, 'info, '_, '_, CrankIrma<'info>>) -> Result<()> {
    let state = ctx.accounts.state.load_mut()?;
    let slots = get_latest_slot()?;

    msg!("Cranking IRMA with pubkey: {:?}", state.pubkey);


    // let lamports: &mut u64 = Box::leak(Box::new(state.lamports));
    // let signer_account_info: &AccountInfo = &ctx.accounts.signer.to_account_info();
    // let system_program: &AccountInfo = &ctx.accounts.system_program.to_account_info();

    let lamports: &mut u64 = Box::leak(Box::new(state.lamports));
    let dummy_info = AccountInfo::new(
        &IRMA_ID,
        false,
        false,
        lamports,
        &mut [],
        &ctx.accounts.system_program.key,
        false,
        0,
    );

    fn allocate_events<'info>() -> &'info mut EventHeap {
        let heap = EventHeap {
            header: EventHeapHeader {
                free_head: 0u16,
                used_head: 0u16,
                count: 0u16,
                _padd: 0u16,
                seq_num: 0u64,
            },
            nodes: [EventNode {
                next: 0u16,
                prev: 0u16,
                _pad: [0u8; 4],
                event: AnyEvent {
                    event_type: 0u8, // Placeholder for event type
                    padding: [0u8; 143], // Placeholder for event data
                },
            }; MAX_NUM_EVENTS as usize],
            reserved: [0u8; 64],
        };
        Box::leak(Box::new(heap))
    }

    let program_id: &'info Pubkey = &IRMA_ID;
    let events_account: Pubkey = Pubkey::find_program_address(&[b"eventheap".as_ref()], program_id).0;
    let lamports: &'info mut u64 = Box::leak(Box::new(100000u64));
    let event_heap: &'info mut EventHeap = allocate_events();

    let events_data: &'info mut [u8] = bytes_of_mut(event_heap);
    let events_key: &'info mut Pubkey = Box::leak(Box::new(events_account));
    msg!("Events account key: {:?}", events_key);

    let events_info: AccountInfo<'info> = AccountInfo::new(
        events_key,
        false,
        false,
        lamports,
        events_data,
        program_id, // owner
        false,
        0,
    );
    let events_info: &AccountInfo<'info> = Box::leak(Box::new(events_info));

    let signer_account_info: AccountInfo<'info> = ctx.accounts.signer.to_account_info();
    let signer_account_info: &AccountInfo<'info> = Box::leak(Box::new(signer_account_info));
    let system_program: AccountInfo<'info> = ctx.accounts.system_program.to_account_info();
    let system_program: &AccountInfo<'info> = Box::leak(Box::new(system_program));

    fn allocate_market<'info>(ekey: Pubkey) -> &'info mut Market {
        let market = Market {
            // PDA bump
            bump: 0u8,
            pad1: [0u8; 7],
            // Number of decimals used for the base token.
            //
            // Used to convert the oracle's price into a native/native price.
            base_decimals: 0u8,
            pad2: [0u8; 7],
            quote_decimals: 0u8,
            pad3: [0u8; 7],
            // padding1: [0u8; 5],

            // Pda for signing vault txs
            market_authority: Pubkey::new_unique(),

            // No expiry = 0. Market will expire and no trading allowed after time_expiry
            time_expiry: 0i64,

            // Admin who can collect fees from the market
            collect_fee_admin: Pubkey::new_unique(),
            // Admin who must sign off on all order creations
            open_orders_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,
            // Admin who must sign off on all event consumptions
            consume_events_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,
            // Admin who can set market expired, prune orders and close the market
            close_market_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,

            // Name. Trailing zero bytes are ignored.
            name: [0u8; 16],

            // Address of the BookSide account for bids
            bids: Pubkey::new_unique(),
            // Address of the BookSide account for asks
            asks: Pubkey::new_unique(),
            // Address of the EventHeap account
            event_heap: ekey,

            // Oracles account address
            oracle_a: Pubkey::new_unique(), // NonZeroPubkeyOption,
            oracle_b: Pubkey::new_unique(), // NonZeroPubkeyOption,
            // Oracle configuration
            oracle_config: OracleConfig {
                conf_filter: 0f64,
                max_staleness_slots: 0i64,
                reserved: [0u8; 72],
            },
            pad4: [0u8; 8],

            // Number of quote native in a quote lot. Must be a power of 10.
            //
            // Primarily useful for increasing the tick size on the market: A lot price
            // of 1 becomes a native price of quote_lot_size/base_lot_size becomes a
            // ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals.
            quote_lot_size: 6i64,

            // Number of base native in a base lot. Must be a power of 10.
            //
            // Example: If base decimals for the underlying asset is 6, base lot size
            // is 100 and and base position lots is 10_000 then base position native is
            // 1_000_000 and base position ui is 1.
            base_lot_size: 6i64,

            // Total number of orders seen
            seq_num: 0u64,

            // Timestamp in seconds that the market was registered at.
            registration_time: 0i64,

            // Fees
            //
            // Fee (in 10^-6) when matching maker orders.
            // maker_fee < 0 it means some of the taker_fees goes to the maker
            // maker_fee > 0, it means no taker_fee to the maker, and maker fee goes to the referral
            maker_fee: -10000i64,
            // Fee (in 10^-6) for taker orders, always >= 0.
            taker_fee: 12000i64,

            // Total fees accrued in native quote
            fees_accrued: 0u128,
            // Total fees settled in native quote
            fees_to_referrers: 0u128,

            // Referrer rebates to be distributed
            referrer_rebates_accrued: 0u64,

            // Fees generated and available to withdraw via sweep_fees
            fees_available: 0u64,

            // Cumulative maker volume (same as taker volume) in quote native units
            maker_volume: 0u128,

            // Cumulative taker volume in quote native units due to place take orders
            taker_volume_wo_oo: 0u128,

            base_mint: Pubkey::from_str_const("BaseMint"), // IRMA mint
            quote_mint: Pubkey::from_str_const("QuoteMint"), // Stablecoin mint

            market_base_vault: Pubkey::new_unique(),
            base_deposit_total: 100u64,

            market_quote_vault: Pubkey::new_unique(),
            quote_deposit_total: 100u64,

            reserved: [0u8; 128],
        };
        Box::leak(Box::new(market))
    }
    
    let market_account: Pubkey = Pubkey::find_program_address(&[b"market".as_ref()], program_id).0;
    let lamports: &'info mut u64 = Box::leak(Box::new(100000u64));
    let market: &'info mut Market = allocate_market(*events_key);

    let market_data: &'info mut [u8] = bytes_of_mut(market);
    let market_key: &'info mut Pubkey = Box::leak(Box::new(market_account));
    msg!("Market account key: {:?}", market_key);

    let market_info: AccountInfo<'info> = AccountInfo::new(
        market_key,
        false,
        false,
        lamports,
        market_data,
        program_id, // owner
        false,
        0,
    );
    let market_info: &AccountInfo<'info> = Box::leak(Box::new(market_info));

    let this_ctx = CpiContext::new(
        dummy_info,
        ConsumeEvents {
            consume_events_admin: Signer::try_from(signer_account_info).unwrap(),
            event_heap: AccountLoader::<'info, EventHeap>::try_from(events_info).unwrap(),
            market: AccountLoader::<'info, Market>::try_from(market_info).unwrap(),
            system_program: Program::try_from(system_program).unwrap(),
        },
    );

    consume_given_events(this_ctx, slots)?;
    Ok(())
}

#[repr(C)]
pub enum OpenBookEvent {
    BuyIRMA {
        trader: Pubkey,
        quote_token: Stablecoins,
        amount: u64,
    },
    SellIRMA {
        trader: Pubkey,
        quote_token: Stablecoins,
        irma_amount: u64,
    },
}

pub fn handle_openbook_event(
    ctx: &Context<IrmaCommon>,
    event: OpenBookEvent,
) -> Result<()> {
    match event {
        OpenBookEvent::BuyIRMA { trader: _, quote_token, amount } => {
            mint_irma(ctx, quote_token, amount)?;
        }
        OpenBookEvent::SellIRMA { trader: _, quote_token, irma_amount } => {
            redeem_irma(ctx, quote_token, irma_amount)?;
        }
    }
    Ok(())
}

pub fn oracle_inflation_input<'info>(
    ctx: &Context<'_, '_, '_, 'info, IrmaCommon<'info>>,
    inflation_percent: f64,
    stablecoin: Stablecoins,
    stablecoin_price_usd: f64,
) -> Result<()> {
    let mint_price = if inflation_percent < 2.0 {
        1.0
    } else {
        stablecoin_price_usd * (1.0 + inflation_percent / 100.0)
    };
    set_mint_price(ctx, stablecoin, mint_price)?;
    Ok(())
}


#[derive(Accounts)]
pub struct CrankIrma<'info> {
    // Add the accounts your crank function needs here
    #[account(init, space = IrmaState::LEN, payer = signer)] // , seeds = [b"irma_state"], bump)]
    pub state: AccountLoader<'info, IrmaState>,
    #[account(mut, signer)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account(zero_copy)]
pub struct IrmaState {
    pub pubkey: Pubkey, // Public key of the account
    pub mint_price: f64,
    pub last_updated: i64, // Timestamp of the last update
    pub lamports: u64, // Lamports for the account
    padding1: [u8; 7], // Padding to align the struct size
    pub stablecoin: u8, // Stablecoin enum value
    padding2: [u8; 7], // Padding to align the struct size
    pub bump: u8, // Bump seed for PDA
}
impl IrmaState {
    pub const LEN: usize = 24 + 32 + 8; // 8 bytes for f64, 8 bytes for i64/u64, and 32 for Pubkey
}

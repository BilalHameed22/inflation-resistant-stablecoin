#![allow(unexpected_cfgs)]
#[cfg(feature = "idl-build")]
// #![feature(trivial_bounds)]
// use std::cmp::{
//     PartialEq,
//     Eq,
// };
// use bytemuck::{
//     Pod,
// };

use anchor_lang::prelude::AccountInfo;
use anchor_lang::prelude::AccountLoader;
use anchor_lang::prelude::Context;
use anchor_lang::prelude::CpiContext;
use anchor_lang::prelude::msg;
use anchor_lang::prelude::Program;
use anchor_lang::prelude::Pubkey;
use anchor_lang::prelude::Rent;
use anchor_lang::prelude::Signer;
use anchor_lang::prelude::System;

use anchor_lang::prelude::*;

use anchor_lang::{
    account,
    Accounts,
    // AnchorSerialize, 
    // AnchorDeserialize, 
    declare_id,
    Discriminator,
    // program,
    pubkey,
    require_keys_neq,
    Result,
    // ToAccountMetas,
    solana_program,
    system_program,
    zero_copy
};
// use anchor_lang::system_program::ID;


pub mod iopenbook;
pub mod pricing;


use iopenbook::{EventHeap, Market, ConsumeEvents, EventHeapHeader, EventNode, AnyEvent, OracleConfig};
use iopenbook::{/*OpenBookV2,*/ get_latest_slot, consume_given_events, MAX_NUM_EVENTS};
use pricing::{
    mint_irma,
    redeem_irma,
    set_mint_price,
    StateMap, 
    MAX_BACKING_COUNT
};

// #[no_mangle]
// unsafe extern "Rust" fn __getrandom_v03_custom(
//     dest: *mut u8,
//     len: usize,
// ) -> Result<(), Error> {
//     Err(Error::UNSUPPORTED)
// }

// impl Default for AHasher {
//     // let mut map: HashMap<i32, i32, BuildHasherDefault<AHasher>> = HashMap::default();
//     // map.insert(13, 53);

//     #[inline]
//     fn default() -> AHasher {
//         RandomState::with_fixed_keys().build_hasher()
//     }
// }

// CPI context and consume_given_events for OpenBook V2
// use anchor_lang::prelude::{AccountInfo, CpiContext, Signer, AccountLoader, Program, Pubkey, AnchorDeserialize, AnchorSerialize};
pub const IRMA_ID: Pubkey = pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");

/// CHECK: following declares unsafe crank_market function - it allocates typed event_heap and typed market that are then
/// serialized into a buffer and then leaked to the static lifetime. Serialized data will be exlusively used to access
/// the OpenBook V2 events and market data. The data is not mutable, so it is safe to leak it to the static lifetime.
#[program]
pub mod irma {
    use super::*;

    /// This is a one-time operation that sets up the IRMA pricing module.
    /// Assume that the markets for the initial IRMA / reserve stablecoin pairs already exist.
    /// This iniatializes only the pricing module for the intial stablecoin reserves, nothing else.
    /// The "Init" data is allocated in a data account that is owned by the IRMA program.
    /// The data is pre-allocated before the call, but empty.
    pub fn initialize(ctx: Context<Init>) -> Result<()> {
        pricing::init_pricing(ctx)
    }

    /// Add a new stablecoin to the reserves.
    /// This is a permissioned instruction that can only be called by the IRMA program owner.
    /// The minimum requirement is that the stablecoin has 100M circulating supply and is not a meme coin.
    /// IRMA relies on pre-existing network effects of each of the reserve stablecoins.
    pub fn add_stablecoin(ctx: Context<Maint>, symbol: String, mint_address: Pubkey, decimals: u8) -> Result<()> {
        pricing::add_stablecoin(ctx, &symbol, mint_address, decimals)
    }

    /// Remove a stablecoin from the reserves by its symbol.
    /// WARNING: This actually removes the stablecoin from the reserves, so be careful when using it.
    /// In order to continue to avoid runs, all reserve amount must be redeemed before removing a stablecoin.
    /// This can be done without using much capital: use 100K IRMAs to redeem another stablecoin (B),
    /// then disable or deactivate the stablecoin to be removed (A), and then do a loop of
    /// 1. internally swapping 100k of stablecoin B for stablecoin A, and then
    /// 2. externally swapping 100k of stablecoin A for 100k of stablecoin B (open market).
    pub fn remove_stablecoin(ctx: Context<Maint>, symbol: String) -> Result<()> {
        pricing::remove_reserve(ctx, &symbol)
    }

    /// Deactivate a reserve stablecoin.
    /// Deactivating should still include the stablecoin in all calculations.
    /// The only action that is disabled should be the minting of IRMA using this reserve stablecoin.
    /// This is done in preparation for removing the stablecoin from the reserves.
    /// For orderly removal, first announce separate dates of deactivation and removal.
    pub fn disable_reserve(ctx: Context<Maint>, symbol: String) -> Result<()> {
        pricing::disable_reserve(ctx, &symbol)
    }

    /// Crank the OpenBook V2 from client.
    /// This function is called periodically (at least once per slot) to process events and update the IRMA state.
    pub fn crank(ctx: Context<Crk>) -> Result<()> {
        crank_market(ctx)
    }
}

/// CHECK: following declares unsafe crank_market function - see comments above.
fn crank_market(ctx: Context<Crk>) -> Result<()> {
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

    fn alloc_heap() -> EventHeap {
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
        return heap;
    }

    // CHECK: following serializes typed object into a buffer.
    let event_heap: EventHeap = alloc_heap();
    let mut event_heap_buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<EventHeap>());
    event_heap.try_serialize(&mut event_heap_buffer).unwrap();
    let boxed_heap: &'static mut Vec<u8> = Box::leak(Box::new(event_heap_buffer));

    let program_id: &'static Pubkey = &IRMA_ID;
    let events_acct: Pubkey = Pubkey::find_program_address(&[b"eventheap".as_ref()], program_id).0;
    let events_key: &'static mut Pubkey = Box::leak(Box::new(events_acct));
    let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));

    msg!("Events account key: {:?}", events_acct);

    let events_info: AccountInfo<'_> = AccountInfo::new(
        events_key,
        false,
        false,
        lamports,
        boxed_heap,
        program_id, // owner
        false,
        0,
    );

    let signer_info: &AccountInfo<'_> = Box::leak(Box::new(ctx.accounts.signer.to_account_info()));
    let sys_program: &AccountInfo<'_> = Box::leak(Box::new(ctx.accounts.system_program.to_account_info()));

    // fn alloc_mkt(ekey: Pubkey) -> Market {
    //     let market = Market {
    //         // PDA bump
    //         bump: 0u8,
    //         pad1: [0u8; 7],
    //         // Number of decimals used for the base token.
    //         //
    //         // Used to convert the oracle's price into a native/native price.
    //         base_decimals: 0u8,
    //         pad2: [0u8; 7],
    //         quote_decimals: 0u8,
    //         pad3: [0u8; 7],
    //         // padding1: [0u8; 5],

    //         // Pda for signing vault txs
    //         market_authority: Pubkey::new_unique(),

    //         // No expiry = 0. Market will expire and no trading allowed after time_expiry
    //         time_expiry: 0i64,

    //         // Admin who can collect fees from the market
    //         collect_fee_admin: Pubkey::new_unique(),
    //         // Admin who must sign off on all order creations
    //         open_orders_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,
    //         // Admin who must sign off on all event consumptions
    //         consume_events_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,
    //         // Admin who can set market expired, prune orders and close the market
    //         close_market_admin: Pubkey::new_unique(), // NonZeroPubkeyOption,

    //         // Name. Trailing zero bytes are ignored.
    //         name: [0u8; 16],

    //         // Address of the BookSide account for bids
    //         bids: Pubkey::new_unique(),
    //         // Address of the BookSide account for asks
    //         asks: Pubkey::new_unique(),
    //         // Address of the EventHeap account
    //         event_heap: ekey,

    //         // Oracles account address
    //         oracle_a: Pubkey::new_unique(), // NonZeroPubkeyOption,
    //         oracle_b: Pubkey::new_unique(), // NonZeroPubkeyOption,
    //         // Oracle configuration
    //         oracle_config: OracleConfig {
    //             conf_filter: 0f64,
    //             max_staleness_slots: 0i64,
    //             reserved: [0u8; 72],
    //         },
    //         pad4: [0u8; 8],

    //         // Number of quote native in a quote lot. Must be a power of 10.
    //         //
    //         // Primarily useful for increasing the tick size on the market: A lot price
    //         // of 1 becomes a native price of quote_lot_size/base_lot_size becomes a
    //         // ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals.
    //         quote_lot_size: 6i64,

    //         // Number of base native in a base lot. Must be a power of 10.
    //         //
    //         // Example: If base decimals for the underlying asset is 6, base lot size
    //         // is 100 and and base position lots is 10_000 then base position native is
    //         // 1_000_000 and base position ui is 1.
    //         base_lot_size: 6i64,

    //         // Total number of orders seen
    //         seq_num: 0u64,

    //         // Timestamp in seconds that the market was registered at.
    //         registration_time: 0i64,

    //         // Fees
    //         //
    //         // Fee (in 10^-6) when matching maker orders.
    //         // maker_fee < 0 it means some of the taker_fees goes to the maker
    //         // maker_fee > 0, it means no taker_fee to the maker, and maker fee goes to the referral
    //         maker_fee: -10000i64,
    //         // Fee (in 10^-6) for taker orders, always >= 0.
    //         taker_fee: 12000i64,

    //         // Total fees accrued in native quote
    //         fees_accrued: 0u128,
    //         // Total fees settled in native quote
    //         fees_to_referrers: 0u128,

    //         // Referrer rebates to be distributed
    //         referrer_rebates_accrued: 0u64,

    //         // Fees generated and available to withdraw via sweep_fees
    //         fees_available: 0u64,

    //         // Cumulative maker volume (same as taker volume) in quote native units
    //         maker_volume: 0u128,

    //         // Cumulative taker volume in quote native units due to place take orders
    //         taker_volume_wo_oo: 0u128,

    //         base_mint: Pubkey::new_from_array([0u8; 32]), //  IRMA mint
    //         quote_mint: Pubkey::new_from_array([0u8; 32]), // Stablecoin mint

    //         market_base_vault: Pubkey::new_unique(),
    //         base_deposit_total: 100u64,

    //         market_quote_vault: Pubkey::new_unique(),
    //         quote_deposit_total: 100u64,

    //         reserved: [0u8; 128],
    //     };
    //     return market;
    // }
    
    // // CHECK: following serializes typed object into a buffer.
    // let market: Market = alloc_mkt(events_acct);
    let mut market_buffer: Vec<u8> = Vec::with_capacity(1024); // std::mem::size_of::<Market>());
    // market.try_serialize(&mut market_buffer).unwrap();
    let boxed_market: &mut Vec<u8> = Box::leak(Box::new(market_buffer));

    let market_acct: Pubkey = Pubkey::find_program_address(&[b"market".as_ref()], program_id).0;
    let market_key: &'static mut Pubkey = Box::leak(Box::new(market_acct));
    let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));

    // msg!("Market account key: {:?}", market_acct);

    let market_info: AccountInfo = AccountInfo::new(
        market_key,
        false,
        false,
        lamports,
        boxed_market,
        program_id, // owner
        false,
        0,
    );

    let this_ctx = CpiContext::new(
        dummy_info,
        ConsumeEvents {
            consume_events_admin: Signer::try_from(signer_info).unwrap(),
            event_heap: events_info,
            market: market_info,
            system_program: Program::try_from(sys_program).unwrap(),
        },
    );

    consume_given_events(this_ctx, slots)?;
    Ok(())
}

#[repr(C)]
enum ObEvent<'a> {
    Buy {
        trader: Pubkey,
        token: &'a str,
        amount: u64,
    },
    Sell {
        trader: Pubkey,
        token: &'a str,
        amount: u64,
    },
}

fn handle_ob_event(
    ctx: Context<Common>,
    event: ObEvent,
) -> Result<()> {
    match event {
        ObEvent::Buy { trader: _, token, amount } => {
            mint_irma(ctx, token, amount)?;
        }
        ObEvent::Sell { trader: _, token, amount } => {
            redeem_irma(ctx, token, amount)?;
        }
    }
    Ok(())
}

fn oracle_input<'info>(
    ctx: Context<'_, '_, '_, 'info, Common<'info>>,
    inflation_percent: f64,
    stablecoin: &str,
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
pub struct Crk<'info> {
    #[account(init, space = State::LEN, payer = signer)]
    pub state: AccountLoader<'info, State>,
    #[account(mut, signer)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account(zero_copy)]
pub struct State {
    pub pubkey: Pubkey,
    pub mint_price: f64,
    pub last_updated: i64,
    pub lamports: u64,
    padding1: [u8; 7],
    pub stablecoin: u8,
    padding2: [u8; 7],
    pub bump: u8,
}
impl State {
    pub const LEN: usize = 24 + 32 + 8;
}


#[derive(Accounts)]
pub struct Init<'info> {
    #[account(init, space=120*MAX_BACKING_COUNT, payer=irma_admin, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, StateMap>,
    #[account(mut)]
    pub irma_admin: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Common<'info> {
    #[account(mut, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, StateMap>,
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Maint<'info> {
    #[account(mut, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, StateMap>,
    #[account(mut)]
    pub irma_admin: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

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
    // declare_program,
    Discriminator,
    // program,
    // Pubkey,
    require_keys_neq,
    Result,
    ToAccountMetas,
    solana_program,
    system_program,
    zero_copy
};
use anchor_lang::solana_program::clock::Clock;
use static_assertions::const_assert_eq;
use std::mem::size_of;
use solana_program::pubkey;


// pub mod iopenbook;
pub mod pricing;
pub const OPENBOOKV2_ID: Pubkey = pubkey!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");

// declare_program!(openbook_v2); // does not work, parsing of IDL file fails
use openbook_v2::state::{EventHeap, Market};
use openbook_v2::cpi::accounts::ConsumeGivenEvents;
// use openbook_v2::accounts::ConsumeGivenEvents;
use openbook_v2::cpi::{consume_events, consume_given_events};
// use openbook_v2::openbook_v2::{consume_given_events};
use openbook_v2::typedefs::{EventHeapHeader, EventNode, AnyEvent, OracleConfig};
// use iopenbook::{EventHeap, Market, ConsumeGivenEvents, EventHeapHeader, EventNode, AnyEvent, OracleConfig};
// use iopenbook::{consume_given_events, MAX_NUM_EVENTS};

use pricing::{
    mint_irma,
    redeem_irma,
    set_mint_price,
    StableState,
    StateMap,
    MAX_BACKING_COUNT
};

// CPI context and consume_given_events for OpenBook V2
// use anchor_lang::prelude::{AccountInfo, CpiContext, Signer, AccountLoader, Program, Pubkey, AnchorDeserialize, AnchorSerialize};
// pub const IRMA_ID: Pubkey = pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
// declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
pub const IRMA_ID: Pubkey = pubkey!("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");
declare_id!("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");

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
        msg!("Add stablecoin entry, size of StateMap: {}", size_of::<StateMap>());
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
    // pub fn crank(ctx: Context<CrankAccounts>) -> Result<()> {
    pub fn crank(ctx: Context<Maint>) -> Result<()> {
        msg!("Crank..., state: {:?}", ctx.accounts.state);
        crank_market(ctx)
    }
}

/// CHECK: following declares unsafe crank_market function - see comments above.
// fn crank_market(ctx: Context<CrankAccounts>) -> Result<()> {
fn crank_market(ctx: Context<Maint>) -> Result<()> {
    msg!("Cranking market...");
    // Get the crank state account and the current slot
    // let state = &ctx.accounts.crank_state;
    // let state = &ctx.accounts.state;
    let slot = 32; // Clock::get().unwrap().slot;
    msg!("Current slot: {}", slot);

    // let clock = Clock::get()?;
    // msg!("Current clock: {:?}", clock);

    // let lamports: &mut u64 = Box::leak(Box::new(state.lamports));
    // let signer_account_info: &AccountInfo = &ctx.accounts.signer.to_account_info();
    // let system_program: &AccountInfo = &ctx.accounts.system_program.to_account_info();

    let lamports: &mut u64 = Box::leak(Box::new(1_000_000u64)); // state.lamports));
    let openbook_info = AccountInfo::new(
        &OPENBOOKV2_ID,
        false,
        false,
        lamports,
        &mut [],
        &ctx.accounts.system_program.key,
        false,
        0,
    );

    msg!("OpenBook V2 ID: {:?}", OPENBOOKV2_ID);

    // fn alloc_heap() -> EventHeap {
    //     let heap = EventHeap {
    //         header: EventHeapHeader {
    //             free_head: 0u16,
    //             used_head: 0u16,
    //             count: 0u16,
    //             padd: 0u16,
    //             seq_num: 0u64,
    //         },
    //         nodes: [EventNode {
    //             next: 0u16,
    //             prev: 0u16,
    //             pad: [0u8; 4],
    //             event: AnyEvent {
    //                 event_type: 0u8, // Placeholder for event type
    //                 padding: [0u8; 143], // Placeholder for event data
    //             },
    //         }; MAX_NUM_EVENTS as usize],
    //         reserved: [0u8; 64],
    //     };
    //     return heap;
    // }

    // CHECK: following serializes typed object into a buffer.
    // let event_heap: EventHeap = alloc_heap();
    let mut event_heap_buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<EventHeap>());
    // event_heap.try_serialize(&mut event_heap_buffer).unwrap();
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

    let irma_admin_info: AccountInfo<'_> = ctx.accounts.irma_admin.to_account_info();
    let sys_program: AccountInfo<'_> = ctx.accounts.system_program.to_account_info();

    // // CHECK: following serializes typed object into a buffer.
    // let market: Market = alloc_mkt(events_acct);
    let market_buffer: Vec<u8> = Vec::with_capacity(1024); // std::mem::size_of::<Market>());
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

    msg!("Market account created: {:?}", market_info.key);

    let this_ctx = CpiContext::new(
        openbook_info,
        ConsumeGivenEvents {
            consume_events_admin: irma_admin_info,
            event_heap: events_info,
            market: market_info,
            // system_program: Program::try_from(sys_program).unwrap(),
        },
    );

    consume_given_events(this_ctx, vec![slot]);
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


/// This data account declaration does not work. Getting the error:
/// Error: Account does not exist or has no data 3ELURJ38nKRf9pdepgvdzXEE9gnPeHHNSpTxH6K3WHqJ (crank_state)
#[derive(Accounts)]
pub struct CrankAccounts<'info> {
    // #[account(init, space = State::LEN, payer = signer)]
    #[account(init, space = 16 + size_of::<State>(), payer=irma_admin, seeds=[b"crank_state".as_ref()], bump)]
    pub crank_state: Account<'info, State>,
    #[account(mut)]
    pub irma_admin: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(PartialEq, Debug)]
pub struct State {
    pub pubkey: Pubkey,
    pub mint_price: f64,
    pub last_updated: i64,
    pub lamports: u64,
    pub stablecoin: u8,
    pub padding1: [u8; 7],
    pub bump: u8,
    pub padding2: [u8; 7],
}
impl State {
    pub const LEN: usize = 32 + 40; // 16 bytes for data type id or discriminator (hidden), total 88 bytes
}

const_assert_eq!(
    size_of::<State>(),
    State::LEN
);

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(init, space=32 + 8 + size_of::<StableState>()*MAX_BACKING_COUNT, payer=irma_admin, seeds=[b"state".as_ref()], bump)]
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
    // pub clock: Sysvar<'info, Clock>,
}


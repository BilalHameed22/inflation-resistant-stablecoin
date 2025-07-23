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

use anchor_lang::solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    // CpiContext,
    // Context,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    program_error::ProgramError,
    system_instruction,
    sysvar::{self, Sysvar},
};

use anchor_lang::prelude::*;


pub mod pricing;


// CPI context and consume_given_events for OpenBook V2
use openbook_v2::ID as OPENBOOKV2_ID;
use openbook_v2::cpi::accounts::ConsumeGivenEvents;
use openbook_v2::cpi::consume_given_events;
use openbook_v2::state::{EventHeap, /* EventNode, AnyEvent, */ Market};
// use openbook_v2::state::EventHeap::MAX_NUM_EVENTS;

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
    pub fn crank(ctx: Context<CrankAccounts>) -> Result<()> {
        crank_market(ctx)
    }
}

/// CHECK: following declares unsafe crank_market function - see comments above.
fn crank_market(ctx: Context<CrankAccounts>) -> Result<()> {
    let openbook = ctx.accounts.openbook_v2.to_account_info();
    let state = ctx.accounts.state.to_account_info(); // load_mut()?;
    // let slots = openbook.clock.slot;

    // msg!("Cranking IRMA with pubkey: {:?}", state.pubkey);


    // let lamports: &mut u64 = Box::leak(Box::new(state.lamports));
    // let signer_account_info: &AccountInfo = &ctx.accounts.signer.to_account_info();
    // let system_program: &AccountInfo = &ctx.accounts.system_program.to_account_info();

    let lamports: &mut u64 = &mut state.lamports.borrow_mut(); // Box::leak(Box::new(state.lamports));

    let openbook_id: Pubkey = Pubkey::new_from_array(OPENBOOKV2_ID.to_bytes());

    // fn alloc_heap() -> EventHeap {
    //     let heap = EventHeap {
    //         header: /* EventHeapHeader */{ SomeStruct {
    //             free_head: 0u16,
    //             used_head: 0u16,
    //             count: 0u16,
    //             _padd: 0u16,
    //             seq_num: 0u64,
    //         } },
    //         nodes: [/* EventNode */ {SomeStruct {
    //             next: 0u16,
    //             prev: 0u16,
    //             _pad: [0u8; 4],
    //             event: /* AnyEvent */ { SomeStruct {
    //                 event_type: 0u8, // Placeholder for event type
    //                 padding: [0u8; 143], // Placeholder for event data
    //             } },
    //         } }; /* MAX_NUM_EVENTS */ 600 as usize],
    //         reserved: [0u8; 64],
    //     };
    //     return heap;
    // }

    // // CHECK: following serializes typed object into a buffer.
    // let event_heap: EventHeap = alloc_heap();

    let mut event_heap_buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<EventHeap>());
    // event_heap.try_serialize(&mut event_heap_buffer).unwrap();
    let boxed_heap: &'static mut Vec<u8> = Box::leak(Box::new(event_heap_buffer));

    let events_acct: Pubkey = Pubkey::find_program_address(&[b"eventheap".as_ref()], &openbook_id).0;
    let events_key: &'static mut Pubkey = Box::leak(Box::new(events_acct));
    let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));

    msg!("Events account key: {:?}", events_acct);

    let events_info: AccountInfo = AccountInfo::new(
        events_key,
        false,
        false,
        lamports,
        boxed_heap,
        &openbook_id, // owner
        false,
        0,
    );

    let signer_info: &AccountInfo<'_> = Box::leak(Box::new(ctx.accounts.signer.to_account_info()));
    let sys_program: &AccountInfo<'_> = Box::leak(Box::new(ctx.accounts.system_program.to_account_info()));
    
    // // CHECK: following serializes typed object into a buffer.
    // let market: Market = alloc_mkt(events_acct);
    let mut market_buffer: Vec<u8> = vec![0u8; std::mem::size_of::<Market>()];
    // market.try_serialize(&mut market_buffer).unwrap();
    let boxed_market: &mut Vec<u8> = Box::leak(Box::new(market_buffer));

    let market_acct: Pubkey = Pubkey::find_program_address(&[b"market".as_ref()], &openbook_id).0;
    let market_key: &'static mut Pubkey = Box::leak(Box::new(market_acct));
    let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));

    // msg!("Market account key: {:?}", market_acct);

    let market_info: AccountInfo = AccountInfo::new(
        market_key,
        false,
        false,
        lamports,
        boxed_market,
        &openbook_id, // owner
        false,
        0,
    );

    // let accounts = vec![
    //     ctx.accounts.signer.to_account_info(),
    //     events_info,
    //     market_info,
    //     sys_program.clone(),
    // ];

    // let cpi_ctx = CpiContext::new(openbook, accounts);

    let this_ctx = CpiContext::new(
        openbook,
        ConsumeGivenEvents {
            consume_events_admin: ctx.accounts.signer.to_account_info(),
            event_heap: events_info,
            market: market_info,
            // system_program: Program::try_from(sys_program).unwrap(),
        },
    );

    let slot = Clock::get()?.slot;

    consume_given_events(this_ctx, vec![slot])
        .map_err(|e| {
            msg!("Error consuming events: {:?}", e);
            e
        });
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
pub struct CrankAccounts<'info> {
    #[account(init, space = State::LEN, payer = signer)]
    pub state: AccountInfo<'info>,
    #[account(mut, signer)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(address = OPENBOOKV2_ID)]
    pub openbook_v2: Program<'info, System>,
}

#[account]
#[derive(PartialEq, Debug)]
pub struct State {
    pub pubkey: Pubkey,
    pub mint_price: f64,
    pub last_updated: i64,
    pub lamports: u64,
    padding1: [u8; 7],
    pub stablecoin: u8,
    // #[account(address = solana_program::sysvar::clock::ID)]
    // pub clock: Clock, // &'static dyn SolanaSysvar,
    // pub sysvar: Sysvar<Clock>,
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

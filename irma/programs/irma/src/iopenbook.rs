
// NOTE: This file is no longer used. Insted, we are linking with the OpenBook V2 program
//
// use solana_sdk_ids::system_program;
// #[cfg(feature = "idl-build")]
use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::prelude::CpiContext;
use anchor_lang::prelude::Program;
// use anchor_lang::prelude::ProgramError;
use anchor_lang::prelude::Pubkey;
use anchor_lang::prelude::Rent;
use anchor_lang::prelude::Signer;
use anchor_lang::prelude::System;
use anchor_lang::prelude::SolanaSysvar;
use anchor_lang::Discriminator;
use anchor_lang::error;
use anchor_lang::Key;
// use anchor_lang::ToAccountInfo;
use anchor_lang_idl_spec::IdlTypeDef;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;
use std::collections::BTreeMap;
use std::mem::size_of;
use std::mem::align_of;
use std::marker::Copy;

pub const MAX_NUM_EVENTS: u16 = 600;
pub const NO_NODE: u16 = u16::MAX;


/// OpenBook V2 interfaces for IRMA program
use std::cmp::{
    PartialEq,
    Eq,
};
use anchor_lang::{
    account,
    Accounts,
    AnchorSerialize, 
    AnchorDeserialize, 
    declare_id,
    error_code,
    require_keys_neq,
    Result,
};

// use crate::iopenbook::*;
// use anchor_lang::solana_program; // ::program_error::ProgramError as SolanaProgError;

// Dummy CPI context and consume_given_events for demonstration
// use anchor_lang::prelude::{AccountInfo, CpiContext, Signer, AccountLoader, Program, Pubkey, AnchorDeserialize, AnchorSerialize};

pub const OPENBOOKV2_ID: Pubkey = pubkey!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");
// Dummy struct for CPI context
pub struct OpenBookV2;

#[derive(Accounts)]
pub struct ConsumeEvents<'info /*, ToAccountInfos, ToAccountMetas */> {
    #[account(
        init,
        // 10240 bytes is max space to allocate with init constraint
        space = 16 + MAX_NUM_EVENTS as usize * (EVENT_SIZE + 8) + 64,
        payer = consume_events_admin,
    )]
    /// CHECK: This uses untyped bytes, validated in the instruction logic.
    pub event_heap: AccountInfo<'info>,
    #[account(mut)]
    pub consume_events_admin: Signer<'info>,
    #[account(
        init,
        // 10240 bytes is max space to allocate with init constraint
        space = 840,
        payer = consume_events_admin,
    )]
    /// CHECK: This uses untyped bytes, validated in the instruction logic.
    pub market: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ConsumeGivenEvents<'info> {
    #[account(mut)]
    pub consume_events_admin: Signer<'info>,
    /// CHECK: This uses untyped bytes, validated in the instruction logic.
    #[account(
        init,
        // 10240 bytes is max space to allocate with init constraint
        space = 16 + MAX_NUM_EVENTS as usize * (EVENT_SIZE + 8) + 64,
        payer = consume_events_admin,
    )]
    pub market: AccountInfo<'info>,
    /// CHECK: This uses untyped bytes, validated in the instruction logic.
    pub event_heap: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn consume_given_events<'info>(
    _ctx: CpiContext<'_, '_, '_, 'info, ConsumeGivenEvents<'info /*, ToAccountInfos, ToAccountMetas */>>, 
    _slots: Vec<u64>
) -> Result<()> {
    Ok(())
}

#[account]
#[derive(PartialEq, Debug)]
pub struct EventHeap {
    pub header: EventHeapHeader,
    pub nodes: [EventNode; MAX_NUM_EVENTS as usize],
    pub reserved: [u8; 64],
}
const_assert_eq!(
    std::mem::size_of::<EventHeap>(),
    16 + MAX_NUM_EVENTS as usize * (EVENT_SIZE + 8) + 64
);


#[account]
#[derive(PartialEq, Debug)]
pub struct OracleConfig {
    pub conf_filter: f64,
    pub max_staleness_slots: i64,
    pub reserved: [u8; 72],
}
const_assert_eq!(size_of::<OracleConfig>(), 8 + 8 + 72);
const_assert_eq!(size_of::<OracleConfig>(), 88);
const_assert_eq!(size_of::<OracleConfig>() % 8, 0);

#[account]
#[derive(PartialEq, Debug)]
pub struct Market {
    /// PDA bump
    pub bump: u8,
    pub pad1: [u8; 7],

    /// Number of decimals used for the base token.
    ///
    /// Used to convert the oracle's price into a native/native price.
    pub base_decimals: u8,
    pub pad2: [u8; 7],
    pub quote_decimals: u8,

    pub pad3: [u8; 7],

    // Pda for signing vault txs
    pub market_authority: Pubkey,

    /// No expiry = 0. Market will expire and no trading allowed after time_expiry
    pub time_expiry: i64,

    /// Admin who can collect fees from the market
    pub collect_fee_admin: Pubkey,
    /// Admin who must sign off on all order creations
    pub open_orders_admin: Pubkey, // NonZeroPubkeyOption,
    /// Admin who must sign off on all event consumptions
    pub consume_events_admin: Pubkey, // NonZeroPubkeyOption,
    /// Admin who can set market expired, prune orders and close the market
    pub close_market_admin: Pubkey, // NonZeroPubkeyOption,

    /// Name. Trailing zero bytes are ignored.
    pub name: [u8; 16],

    /// Address of the BookSide account for bids
    pub bids: Pubkey,
    /// Address of the BookSide account for asks
    pub asks: Pubkey,
    /// Address of the EventHeap account
    pub event_heap: Pubkey,

    /// Oracles account address
    pub oracle_a: Pubkey, // NonZeroPubkeyOption,
    pub oracle_b: Pubkey, // NonZeroPubkeyOption,
    /// Oracle configuration
    pub oracle_config: OracleConfig,
    pub pad4: [u8; 8],

    /// Number of quote native in a quote lot. Must be a power of 10.
    ///
    /// Primarily useful for increasing the tick size on the market: A lot price
    /// of 1 becomes a native price of quote_lot_size/base_lot_size becomes a
    /// ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals.
    pub quote_lot_size: i64,

    /// Number of base native in a base lot. Must be a power of 10.
    ///
    /// Example: If base decimals for the underlying asset is 6, base lot size
    /// is 100 and and base position lots is 10_000 then base position native is
    /// 1_000_000 and base position ui is 1.
    pub base_lot_size: i64,

    /// Total number of orders seen
    pub seq_num: u64,

    /// Timestamp in seconds that the market was registered at.
    pub registration_time: i64,

    /// Fees
    ///
    /// Fee (in 10^-6) when matching maker orders.
    /// maker_fee < 0 it means some of the taker_fees goes to the maker
    /// maker_fee > 0, it means no taker_fee to the maker, and maker fee goes to the referral
    pub maker_fee: i64,
    /// Fee (in 10^-6) for taker orders, always >= 0.
    pub taker_fee: i64,

    /// Total fees accrued in native quote
    pub fees_accrued: u128,
    /// Total fees settled in native quote
    pub fees_to_referrers: u128,

    /// Referrer rebates to be distributed
    pub referrer_rebates_accrued: u64,

    /// Fees generated and available to withdraw via sweep_fees
    pub fees_available: u64,

    /// Cumulative maker volume (same as taker volume) in quote native units
    pub maker_volume: u128,

    /// Cumulative taker volume in quote native units due to place take orders
    pub taker_volume_wo_oo: u128,

    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,

    pub market_base_vault: Pubkey,
    pub base_deposit_total: u64,

    pub market_quote_vault: Pubkey,
    pub quote_deposit_total: u64,

    pub reserved: [u8; 128],
}

const_assert_eq!(
    size_of::<Market>(),        // 864
    8 +                         // discriminator (hidden)
    32 +                        // market_authority
    32 +                        // collect_fee_admin
    32 +                        // open_order_admin
    32 +                        // consume_event_admin
    32 +                        // close_market_admin
    1 +                         // bump
    7 +
    1 +                         // base_decimals
    7 +
    1 +                         // quote_decimals
    7 +                         // pad3
    8 +                         // time_expiry
    16 +                        // name
    3 * 32 +                    // bids, asks, and event_heap
    32 +                        // oracle_a
    32 +                        // oracle_b
    size_of::<OracleConfig>() + // oracle_config 88 bytes
    8 +                         // quote_lot_size
    8 +                         // base_lot_size
    8 +                         // seq_num
    8 +                         // registration_time
    8 +                         // maker_fee
    8 +                         // taker_fee
    16 +                        // fees_accrued
    16 +                        // fees_to_referrers
    16 +                        // maker_volume
    16 +                        // taker_volume_wo_oo
    4 * 32 +                    // base_mint, quote_mint, market_base_vault, and market_quote_vault
    8 +                         // base_deposit_total
    8 +                         // quote_deposit_total
    8 +                         // base_fees_accrued
    8 +                         // referrer_rebates_accrued
    128 // reserved
);
// const_assert_eq!(size_of::<Market>(), 848);
const_assert_eq!(size_of::<Market>() % 8, 0);


#[derive(
    Eq,
    PartialEq,
    Debug,
)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}


#[account]
#[derive(PartialEq, Debug)]
pub struct EventHeapHeader {
    pub free_head: u16,
    pub used_head: u16,
    pub count: u16,
    pub _padd: u16,
    pub seq_num: u64,
}
const_assert_eq!(std::mem::size_of::<EventHeapHeader>(), 16);
const_assert_eq!(std::mem::size_of::<EventHeapHeader>() % 8, 0);

#[account]
#[derive(PartialEq, Copy, Debug)]
pub struct EventNode {
    pub next: u16,
    pub prev: u16,
    pub _pad: [u8; 4],
    pub event: AnyEvent,
}
const_assert_eq!(std::mem::size_of::<EventNode>(), 8 + EVENT_SIZE);
const_assert_eq!(std::mem::size_of::<EventNode>() % 8, 0);

impl EventNode {
    pub fn is_free(&self) -> bool {
        self.prev == NO_NODE
    }
}

const EVENT_SIZE: usize = 144;
#[account]
#[derive(PartialEq, Copy, Debug)]
pub struct AnyEvent {
    pub event_type: u8,
    pub padding: [u8; 143],
}

const_assert_eq!(size_of::<AnyEvent>(), EVENT_SIZE);

#[derive(
    Eq,
    PartialEq,
    Debug,
)]
#[repr(u8)]
pub enum EventType {
    Fill,
    Out,
}

#[account]
#[derive(PartialEq, Debug)]
#[repr(C)]
pub struct FillEvent {
    pub event_type: u8,
    pub taker_side: u8, // Side, from the taker's POV
    pub maker_out: u8,  // 1 if maker order quantity == 0
    pub maker_slot: u8,
    pub padding: [u8; 4],
    pub timestamp: u64,
    pub market_seq_num: u64,

    pub maker: Pubkey,

    // Timestamp of when the maker order was placed; copied over from the LeafNode
    pub maker_timestamp: u64,

    pub taker: Pubkey,
    pub taker_client_order_id: u64,

    pub price: i64,
    pub peg_limit: i64,
    pub quantity: i64, // number of base lots
    pub maker_client_order_id: u64,
    pub reserved: [u8; 8],
}
const_assert_eq!(size_of::<FillEvent>() % 8, 0);
const_assert_eq!(size_of::<FillEvent>(), EVENT_SIZE);

#[account]
#[derive(PartialEq, Debug)]
#[repr(C)]
pub struct OutEvent {
    pub event_type: u8,
    pub side: u8, // Side
    pub owner_slot: u8,
    padding0: [u8; 5],
    pub timestamp: u64,
    pub seq_num: u64,
    pub owner: Pubkey,
    pub quantity: i64,
    padding1: [u8; 80],
}
const_assert_eq!(size_of::<OutEvent>() % 8, 0);
const_assert_eq!(size_of::<OutEvent>(), EVENT_SIZE);

// From OpenBook V2 order_type.rs

#[derive(PartialEq, Debug)]
pub enum BookSideOrderTree {
    Fixed = 0,
    OraclePegged = 1,
}

#[derive(
    Eq,
    PartialEq,
    Debug,
)]
#[repr(u8)]
pub enum PlaceOrderType {
    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// If any base_quantity or quote_quantity remains, place an order on the book
    Limit = 0,

    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// Never place an order on the book.
    ImmediateOrCancel = 1,

    /// Never take any existing orders, post the order on the book if possible.
    /// If existing orders can match with this order, do nothing.
    PostOnly = 2,

    /// Ignore price and take orders up to max_base_quantity and max_quote_quantity.
    /// Never place an order on the book.
    ///
    /// Equivalent to ImmediateOrCancel with price=i64::MAX.
    Market = 3,

    /// If existing orders match with this order, adjust the price to just barely
    /// not match. Always places an order on the book.
    PostOnlySlide = 4,

    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// Abort if partially executed, never place an order on the book.
    FillOrKill = 5,
}

#[derive(
    Eq,
    PartialEq,
    Debug,
)]
#[repr(u8)]
pub enum PostOrderType {
    /// Take existing orders up to price, max_base_quantity and max_quote_quantity.
    /// If any base_quantity or quote_quantity remains, place an order on the book
    Limit = 0,

    /// Never take any existing orders, post the order on the book if possible.
    /// If existing orders can match with this order, do nothing.
    PostOnly = 2,

    /// If existing orders match with this order, adjust the price to just barely
    /// not match. Always places an order on the book.
    PostOnlySlide = 4,
}

#[derive(
    Eq,
    PartialEq,
    Debug,
    Default
)]
#[repr(u8)]
/// Self trade behavior controls how taker orders interact with resting limit orders of the same account.
/// This setting has no influence on placing a resting or oracle pegged limit order that does not match
/// immediately, instead it's the responsibility of the user to correctly configure his taker orders.
pub enum SelfTradeBehavior {
    /// Both the maker and taker sides of the matched orders are decremented.
    /// This is equivalent to a normal order match, except for the fact that no fees are applied.
    #[default]
    DecrementTake = 0,

    /// Cancels the maker side of the trade, the taker side gets matched with other maker's orders.
    CancelProvide = 1,

    /// Cancels the whole transaction as soon as a self-matching scenario is encountered.
    AbortTransaction = 2,
}

/// SideAndOrderTree is a storage optimization, so we don't need two bytes for the data
#[derive(
    Eq,
    PartialEq,
    Debug,
)]
#[repr(u8)]
pub enum SideAndOrderTree {
    BidFixed = 0,
    AskFixed = 1,
    BidOraclePegged = 2,
    AskOraclePegged = 3,
}


#[error_code]
pub enum OpenBookError {
    #[msg("Invalid order post-market provided.")]
    InvalidOrderPostMarket,
    #[msg("Invalid order post-only provided.")]
    InvalidOrderPostIOC,
}

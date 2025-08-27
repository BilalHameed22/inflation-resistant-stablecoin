#![allow(unexpected_cfgs)]
// #![feature(trivial_bounds)]
// #[cfg(feature = "idl-build")]
// use std::string::String;
// se std::vec::Vec;
// use std::option::Option;
// use anchor_lang_idl_spec::IdlType::Option as IdlOption;
// use anchor_lang_idl_spec::IdlType::Pubkey as IdlPubkey;

use anchor_lang::*;
use anchor_lang::system_program::ID as SYS_PROGRAM_ID;
use anchor_lang::prelude::*;
use Vec;
// use std::collections::BTreeMap;
// use static_assertions::const_assert_eq;
// use core::mem::size_of;
// use solana_program::pubkey;
use solana_program::pubkey;

use crate::pricing::{
    mint_irma,
    redeem_irma,
    set_mint_price,
    // MAX_BACKING_COUNT,
    // Init,
    Common,
    Maint,
    // StateMap,
};
use crate::IRMA_ID;
// use crate::OPENBOOKV2_ID;

use crate::iopenbook::{ConsumeEvents, Market, EventHeap, EventHeapHeader, EventNode, AnyEvent, OracleConfig};
// use iopenbook::ConsumeGivenEvents;

pub const OPENBOOKV2_ID: Pubkey = pubkey!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");

use openbook_v2::state::EventHeap; // {EventHeap, Market};
// use openbook_v2::cpi::{consume_events, consume_given_events};
use openbook_v2::typedefs::{EventHeapHeader, EventNode, AnyEvent, OracleConfig};
// use openbook_v2::ix_accounts::{ConsumeEvents, PlaceOrder};
use openbook_v2::cpi::accounts::{ConsumeGivenEvents, PlaceOrder};
use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::Ref;


/// CHECK: following declares unsafe crank_market function - see comments above.
/// CPI context and consume_given_events for OpenBook V2
// pub fn crank_market<'c: 'info, 'info>( ctx: Context<'_, '_, 'c, 'info, ConsumeEvents>, slot: u64 ) -> Result<()> {
pub fn crank_market<'info>( ctx: Context<'_, '_, 'info, 'info, Maint<'info>>, slot: u64 ) -> Result<()> {

    msg!("Crank market called with slot: {}", slot);

    fn prep_accounts<'info>(
        ctx: Context::<'_, '_, 'info, 'info, Maint<'info>>, 
        owner: &'info Pubkey, 
        state_account: Pubkey
    ) -> (
        AccountInfo<'info>,
        AccountInfo<'info>,
        AccountInfo<'info>,
        AccountInfo<'info>
    ) {
        // let signer_account_info: &AccountInfo = &ctx.accounts.signer.to_account_info();
        // let system_program: &AccountInfo = &ctx.accounts.system_program.to_account_info();
        msg!("Preparing accounts for crank market...");

        let lamports_ob: &mut u64 = Box::leak(Box::new(1_000_001u64));
        let lamports_event: &mut u64 = Box::leak(Box::new(1_000_002u64));
        let lamports: &mut u64 = Box::leak(Box::new(1_000_003u64));

        let openbook_info = AccountInfo::<'info>::new(
            &OPENBOOKV2_ID,
            false,
            false,
            lamports_ob,
            &mut [],
            &SYS_PROGRAM_ID, // ctx.accounts.system_program.key,
            false,
            0,
        );

        msg!("OpenBook V2 ID: {:?}", OPENBOOKV2_ID);
        msg!("OpenBook account created: {:?}", openbook_info.key);

        // CHECK: following serializes typed object into a buffer.
        // let event_heap: EventHeap = alloc_heap();
        const BUF_SIZE: usize = 1600; // std::mem::size_of::<EventHeap>();
        msg!("Allocating event heap, mem size: {}", BUF_SIZE);
        let event_heap_buffer = [0u8; BUF_SIZE]; // Vec::with_capacity(BUF_SIZE);
        let event_heap_ref: &'info mut [u8] = Box::leak(Box::new(event_heap_buffer)); // = &mut event_heap_buffer;

        let program_id = &IRMA_ID;
        let events_acct = Box::leak(Box::new(Pubkey::find_program_address(&[b"eventheap".as_ref()], program_id).0));
        // let state = &mut ctx.accounts.crank_state;

        msg!("Events account key: {:?}", events_acct);

        let events_info = AccountInfo::<'info>::new(
            events_acct,
            false,
            false,
            lamports_event,
            event_heap_ref,
            program_id, // owner
            false,
            0,
        );
        msg!("EventHeap account created: {:?}", events_info.key);

        let signer_pubkey: &'info mut Pubkey = Box::leak(Box::new(*ctx.accounts.irma_admin.key));
        let lamportsx: &'info mut u64 = Box::leak(Box::new(0u64));
        let data: &'info mut Vec<u8> = Box::leak(Box::new(vec![]));
        let owner: &'info mut Pubkey = Box::leak(Box::new(Pubkey::default()));
        let signer_account_info: AccountInfo<'info> = AccountInfo::new(
            signer_pubkey,
            true, // is_signer
            false, // is_writable
            lamportsx,
            data,
            owner,
            false,
            0,
        );

        // // CHECK: following serializes typed object into a buffer.
        // let market: Market = alloc_mkt(events_acct);
        let market_buffer = vec![0u8; 1024]; // std::mem::size_of::<Market>()];
        let market_buf_ref: &'info mut Vec<u8> = Box::leak(Box::new(market_buffer)); // &mut market_buffer;

        let market_acct: &Pubkey = Box::leak(Box::new(Pubkey::find_program_address(&[b"market".as_ref()], program_id).0));
        // let state = &mut ctx.accounts.crank_state;

        let market_info = AccountInfo::<'info>::new(
            market_acct,
            false,
            false,
            lamports,
            market_buf_ref,
            program_id, // owner
            false,
            0,
        );

        msg!("Market account created: {:?}", market_info.key);

        (openbook_info, events_info, market_info, signer_account_info)
    }

    let sys_lamports: &'info mut u64 = Box::leak(Box::new(0u64));
    let sys_data: &'info mut Vec<u8> = Box::leak(Box::new(vec![]));
    let sys_owner: &'info mut Pubkey = Box::leak(Box::new(Pubkey::default()));
    let sys_account_info: AccountInfo<'info> = AccountInfo::new(
        &system_program::ID,
        false, // is_signer
        false, // is_writable
        sys_lamports,
        sys_data,
        sys_owner,
        true,
        0,
    );

    let program_id = &IRMA_ID;
    let state_account: Pubkey = Pubkey::find_program_address(&[b"crank_state".as_ref()], program_id).0;
    let (openbook_info, events_info, market_info, signer_account_info) = prep_accounts(ctx, program_id, state_account);

    let mut this_ctx = CpiContext::<'_, '_, 'info, 'info, ConsumeEvents<'info>>::new(
        openbook_info,
        ConsumeEvents {
            consume_events_admin: Signer::try_from(&signer_account_info).unwrap(),
            event_heap: Account::try_from(&events_info).unwrap(),
            market: Account::try_from(&market_info).unwrap(),
            system_program: Program::try_from(&sys_account_info).unwrap(),
        },
    );

    msg!("CPI context prepared for consume events");

    #[cfg(not(test))]
    {
        struct OurHeap {
            header: EventHeapHeader,
            nodes: [EventNode; 10], // Reduced size for testing
            reserved: [u8; 64],
        }
        impl OurHeap {
            fn try_to_vec(&self) -> Result<Vec<u8>> {
                let mut buf = Vec::with_capacity(std::mem::size_of::<OurHeap>());
                buf.extend_from_slice(&self.header.try_to_vec()?);
                for node in &self.nodes {
                    buf.extend_from_slice(&node.try_to_vec()?);
                }
                buf.extend_from_slice(&self.reserved);
                Ok(buf)
            }
        }

        fn alloc_heap() -> Box<OurHeap> {
            let mut heap = Box::new(OurHeap {
                header: EventHeapHeader {
                    free_head: 1u16,
                    used_head: 0u16,
                    count: 1u16,
                    _padd: 0u16,
                    seq_num: 1u64,
                },
                nodes: [EventNode {
                    next: 0u16,
                    prev: 0u16,
                    _pad: [0u8; 4],
                    event: AnyEvent {
                        event_type: 0u8, // Placeholder for event type
                        padding: [0u8; 143], // Placeholder for event data
                    },
                }; 10 as usize], // just the first 10 events
                reserved: [0u8; 64],
            });
            heap.nodes[0].event.event_type = 1; // Set the first event type to 1
            return heap;
        }

        fn consume_given_events_mock<'info>(ctx: &CpiContext<'_, '_, 'info, 'info, ConsumeEvents<'info>>, _slots: Vec<u64>) {
            // mock implementation
            msg!("Mocking consume_given_events with slots: {:?}", _slots);
            let event_heap = alloc_heap();
            msg!("Mocked event heap header: {:?}", &event_heap.header);

            // let market_info: AccountInfo<'a> = ctx.accounts.market.to_account_info();
            // msg!("Market account key: {:?}", market_info.key);
            // let market: Market = alloc_mkt(market_info.key);
            // msg!("Mocked market: {:?}", market);
            let binding = ctx.accounts.event_heap.to_account_info();
            let mut event_heap_buf = binding.data.borrow_mut();
            let BUF_SIZE: usize = 1600; // = std::mem::size_of::<EventHeap>();
            if event_heap_buf.len() < BUF_SIZE {
                msg!("Event heap buffer too small, mock returning...");
                return;
            }
            msg!("In mock execution, heap size: {}", event_heap_buf[..].len());
            // serialized is our initialized data, while event_heap_buf is from outside, attached to ctx.
            let serialized = event_heap.try_to_vec().unwrap();
            if serialized.len() <= event_heap_buf.len() {
                msg!("Serialized event heap size: {}", serialized.len());
                event_heap_buf[..serialized.len()].copy_from_slice(&serialized);
            } else {
                msg!("Serialized data too large: {} > {}", serialized.len(), event_heap_buf.len());
            }
            // let mut market_buf = ctx.accounts.market.to_account_info().data.borrow_mut();
            // market_buf.clear();
            // let mut cursor = Cursor::new(&mut market_buf[..]); // Create a Cursor from the slice
            // market.try_serialize(&mut cursor).unwrap();
            let header_size = std::mem::size_of::<EventHeapHeader>();
            msg!("Mocked consume_given_events about to return, event heap updated: {:?}", &event_heap_buf[..header_size]);
        }

        msg!("Calling consume_given_events_mock...");
        let ctx = &this_ctx;
        consume_given_events_mock(ctx, vec![slot]);
    }
    // #[cfg(test)]
    // {
    //     msg!("Calling consume_given_events...");
    //     let ctx = this_ctx.borrow();
    //     openbook_v2::cpi::consume_given_events(ctx, vec![slot]);
    // }
    let binding = this_ctx.accounts.event_heap.to_account_info();
    let event_heap_buf = <Rc<RefCell<&mut [u8]>> as Borrow<RefCell<&mut [u8]>>>::borrow(&binding.data).borrow();
    msg!("consume_given_events completed, event heap updated: {}", event_heap_buf[0]);

    Ok(())
}


#[repr(C)]
enum ObEvent<'info> {
    Buy {
        trader: Pubkey,
        token: &'info str,
        amount: u64,
    },
    Sell {
        trader: Pubkey,
        token: &'info str,
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

fn oracle_input<'c: 'info,'info>(
    ctx: Context<'_, '_, 'c, 'info, Common<'info>>,
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


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
use anchor_lang::prelude::Context;
// use anchor_lang::prelude::CpiContext;
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
    // Discriminator,
    // program,
    // Pubkey,
    // require_keys_neq,
    Result,
    // ToAccountMetas,
    solana_program,
    system_program,
    // zero_copy
};
// se anchor_lang::solana_program::clock::Clock;
// use static_assertions::const_assert_eq;
// use std::io::{Cursor, Read, Write};
use std::mem::size_of;
use std::collections::BTreeMap;
use solana_program::pubkey;
// use borsh::BorshSerialize; // Add this import


pub mod pricing;

use crate::pricing::*;

pub use crate::pricing::{StateMap, StableState, Init, Common, Maint};

pub mod crank_market;
pub mod iopenbook;

pub use crate::crank_market::{
    crank_market,
};

// pub use crate::iopenbook::{ConsumeEvents, Market, EventHeap, EventHeapHeader, EventNode, AnyEvent, OracleConfig};


// use anchor_lang::prelude::{AccountInfo, CpiContext, Signer, AccountLoader, Program, Pubkey, AnchorDeserialize, AnchorSerialize};
// pub const IRMA_ID: Pubkey = pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
// declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");
pub const IRMA_ID: Pubkey = pubkey!("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");
declare_id!("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");

/// IRMA program
/// Use OpenBook V2 to process events and update the IRMA state, including pricing.
#[program]
pub mod irma {
    use super::*;

    /// This is a one-time operation that sets up the IRMA pricing module.
    /// Assume that the markets for the initial IRMA / reserve stablecoin pairs already exist.
    /// This iniatializes only the pricing module for the intial stablecoin reserves, nothing else.
    /// The "Init" data is allocated in a data account that is owned by the IRMA program.
    /// The data is pre-allocated before the call, but empty.
    pub fn initialize(ctx: Context<Init>) -> Result<()> {
        crate::pricing::init_pricing(ctx)
    }

    /// Add a new stablecoin to the reserves.
    /// This is a permissioned instruction that can only be called by the IRMA program owner.
    /// The minimum requirement is that the stablecoin has 100M circulating supply and is not a meme coin.
    /// IRMA relies on pre-existing network effects of each of the reserve stablecoins.
    pub fn add_reserve(ctx: Context<Maint>, symbol: String, mint_address: Pubkey, decimals: u8) -> Result<()> {
        msg!("Add stablecoin entry, size of StateMap: {}", size_of::<StateMap>());
        crate::pricing::add_reserve(ctx, &symbol, mint_address, decimals)
    }

    /// Remove a stablecoin from the reserves by its symbol.
    /// WARNING: This actually removes the stablecoin from the reserves, so be careful when using it.
    /// In order to continue to avoid runs, all reserve amount must be redeemed before removing a stablecoin.
    /// This can be done without using much capital: use 100K IRMAs to redeem another stablecoin (B),
    /// then disable or deactivate the stablecoin to be removed (A), and then do a loop of
    /// 1. internally swapping 100k of stablecoin B for stablecoin A, and then
    /// 2. externally swapping 100k of stablecoin A for 100k of stablecoin B (open market).
    pub fn remove_reserve(ctx: Context<Maint>, symbol: String) -> Result<()> {
        crate::pricing::remove_reserve(ctx, &symbol)
    }

    /// Deactivate a reserve stablecoin.
    /// Deactivating should still include the stablecoin in all calculations.
    /// The only action that is disabled should be the minting of IRMA using this reserve stablecoin.
    /// This is done in preparation for removing the stablecoin from the reserves.
    /// For orderly removal, first announce separate dates of deactivation and removal.
    pub fn disable_reserve(ctx: Context<Maint>, symbol: String) -> Result<()> {
        crate::pricing::disable_reserve(ctx, &symbol)
    }

    // Crank the OpenBook V2 from client.
    // This function is called periodically (at least once per slot) to process events and update the IRMA state.
    // pub fn crank<'c: 'info, 'info>(ctx: Context<'_, '_, 'c, 'info, ConsumeEvents>) -> Result<()> {
    pub fn crank(_dummy: Context<Maint>) -> Result<()> {
        msg!("Crank..., ");
        let slot;
        #[cfg(not(test))]
        {
            msg!("Crank in test mode, mocking slot number...");
            slot = 1223312; // Mock slot for testing
        }
        #[cfg(test)]
        {
            slot = Clock::get()?.slot;
        }
        msg!("Current slot: {}", slot);

        // Create a buffer for StateMap and wrap it in AccountInfo
        let state_account = Pubkey::find_program_address(&[b"state".as_ref()], &IRMA_ID).0;
        let lamports: &mut u64 = &mut Box::new(100000u64);
        let mut state: StateMap = StateMap::new();
        let _ = state.init_reserves(); // Add initial stablecoins to the state

        // Prepare the account data with the correct discriminator
        let mut state_data_vec: Vec<u8> = Vec::with_capacity(120*MAX_BACKING_COUNT);
        state.try_serialize(&mut state_data_vec).unwrap();

        let state_data: &mut Vec<u8> = &mut Box::new(state_data_vec);
        let state_key: &mut Pubkey = &mut Box::new(state_account);
        let owner: &Pubkey = &mut Box::new(IRMA_ID);
        // msg!("StateMap pre-test account data: {:?}", state_data);
        let state_account_info: AccountInfo<'_> = AccountInfo::new(
            state_key,
            false, // is_signer
            true,  // is_writable
            lamports,
            state_data,
            owner,
            false,
            0,
        );
        // msg!("StateMap account created: {:?}", state_account_info.key);
        // msg!("StateMap owner: {:?}", owner);
        // Use a mock Signer for testing purposes
        // let signer_pubkey: &'info mut Pubkey = &mut Box::new(Pubkey::new_unique())); // causes ELF error!
        let lamportsx: &mut u64 = &mut Box::new(0u64);
        let data: &mut Vec<u8> = &mut Box::new(vec![]);
        let system_id = system_program::ID;
        let owner: &mut Pubkey =  &mut Box::new(system_id);
        let signer_account_info: AccountInfo<'_> = AccountInfo::new(
            owner, // signer_pubkey,
            true, // is_signer
            false, // is_writable
            lamportsx,
            data,
            owner,
            false,
            0,
        );
        // Create AccountInfo for system_program
        let sys_lamports: &mut u64 = &mut Box::new(0u64);
        let sys_data: &mut Vec<u8> = &mut Box::new(vec![]);
        let sys_owner: &mut Pubkey = &mut Box::new(Pubkey::default());
        let sys_account_info: AccountInfo<'_> = AccountInfo::new(
            &system_program::ID,
            false, // is_signer
            false, // is_writable
            sys_lamports,
            sys_data,
            sys_owner,
            true,
            0,
        );

        let mut bumps = BTreeMap::<String, u8>::new();
        bumps.insert("state".to_string(), 13u8);
        bumps.insert("irma_admin".to_string(), 13u8);
        bumps.insert("system_program".to_string(), 13u8);

        let ctx = Context::<'_, '_, '_, '_, Maint<'_>> {
            // Fill in the context with necessary accounts and data
            // This is a placeholder, actual implementation will depend on the accounts structure
            accounts: &mut Maint {
                state: Account::try_from(&state_account_info).unwrap(),
                irma_admin: Signer::try_from(&signer_account_info).unwrap(),
                system_program: Program::try_from(&sys_account_info).unwrap(),
            },
            remaining_accounts: &[],
            program_id: &IRMA_ID,
            bumps,
        };
        
        msg!("Cranking market...");
        
        crank_market(ctx, slot)
    }
}



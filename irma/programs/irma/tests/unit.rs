
#[cfg(test)]
mod tests {
    use anchor_lang::prelude::*;
    use anchor_lang::prelude::Pubkey;
    use solana_sdk_ids::system_program;
    use anchor_lang::prelude::Signer;
    // use anchor_lang::prelude::Account;
    use anchor_lang::prelude::Program;
    use anchor_lang::context::Context;
    // use bytemuck::bytes_of_mut;
    // use anchor_lang::Discriminator;
    use irma_program::IRMA_ID;
    use irma_program::pricing::CustomError;
    use irma_program::pricing::{StateMap, StableState, Initialize, IrmaCommon, IrmaCommonBumps, InitializeBumps};
    use irma_program::pricing::{initialize, set_mint_price, mint_irma, redeem_irma};
    use irma_program::pricing::MAX_BACKING_COUNT;

    
    fn allocate_state() -> StateMap {
        StateMap::new()
    }

    fn init_state() -> StateMap {
        let mut state: StateMap = allocate_state();
        let reserves = &mut state.reserves;
        let usdt: StableState = StableState::new("USDT", pubkey!("Es9vMFrzaTmVRL3P15S3BtQDvVwWZEzPDk1e45sA2v6p"), 6 as u64).unwrap();
        reserves.insert("USDT".to_string(), usdt);
        assert_eq!(reserves.len(), 1);
        state
    }

    #[test]
    fn test_set_state_directly() {
        let mut state: StateMap = init_state();
        let quote_token: &str = "USDT";
        let new_price: f64 = 1.23;
        {
            let reserves = &mut state.reserves;
            let mut_reserve = reserves.get_mut(quote_token).unwrap();
            // assert_eq!(mut_reserve.mint_price, 1.0);
            mut_reserve.mint_price = 1.0;
        }
        {
            let reserves = state.reserves.clone();
            assert_eq!(reserves[quote_token].mint_price, 1.0);
        }
        {
            let reserves = &mut state.reserves;
            let mut_reserve = reserves.get_mut(quote_token).unwrap();
            mut_reserve.mint_price = new_price;
        }
        let reserves = state.reserves;
        assert_eq!(reserves[quote_token].mint_price, new_price);
    }

    #[test]
    fn test_mint_irma_directly() {
        let mut state = init_state();
        let reserves = &mut state.reserves;
        let quote_token = "USDT";
        let amount = 100;
        let price = reserves[quote_token].mint_price;
        let prev_circulation = reserves[quote_token].irma_in_circulation;
        let prev_reserve = reserves[quote_token].backing_reserves;
        // Simulate mint_irma logic
        let mut_reserve = reserves.get_mut(quote_token).unwrap();
        mut_reserve.backing_reserves += amount;
        mut_reserve.irma_in_circulation += (amount as f64 / price).ceil() as u64;
        assert_eq!(reserves[quote_token].backing_reserves, prev_reserve + amount);
        assert_eq!(reserves[quote_token].irma_in_circulation, prev_circulation + (amount as f64 / price).ceil() as u64);
    }

    #[test]
    fn test_redeem_irma_simple() {
        let mut state = init_state();
        let reserves = &mut state.reserves;
        let quote_token = "USDT";
        {
            let mut_reserve = reserves.get_mut(quote_token).unwrap();
            mut_reserve.backing_reserves = 1000;
        }
        let prevBacking = reserves[quote_token].backing_reserves;
        {
            let mut_reserve = reserves.get_mut(quote_token).unwrap();
            mut_reserve.backing_reserves -= 100;
        }
        // Simulate redeem_irma logic (simple case)
        assert_eq!(reserves[quote_token].backing_reserves, prevBacking - 100);
    }

    #[test]
    fn test_reduce_circulations_logic() {
        let mut state = init_state();
        let reserves = &mut state.reserves;
        let prev_circulation = 100; // reserves["USDT"].irma_in_circulation;
        let irma_amount = 5;
        {
            // Manipulate state to create a price difference
            let mut_reserve = reserves.get_mut("USDT").unwrap();
            mut_reserve.mint_price = 2.0;
            mut_reserve.backing_reserves = 1000;
            mut_reserve.irma_in_circulation = 100;
            mut_reserve.irma_in_circulation -= irma_amount;
        }
        assert_eq!(reserves["USDT"].irma_in_circulation, prev_circulation - irma_amount);
    }

    fn prep_accounts(owner: &'static Pubkey, state_account: Pubkey) -> (AccountInfo<'static>, AccountInfo<'static>, AccountInfo<'static>) {
        // Create a buffer for StateMap and wrap it in AccountInfo
        let lamports: &mut u64 = Box::leak(Box::new(100000u64));
        let mut state: StateMap = allocate_state();
        let _ = state.add_initial_stablecoins(); // Add initial stablecoins to the state

        // Prepare the account data with the correct discriminator
        let mut state_data_vec: Vec<u8> = Vec::with_capacity(120*MAX_BACKING_COUNT);
        state.try_serialize(&mut state_data_vec).unwrap();

        let state_data: &'static mut Vec<u8> = Box::leak(Box::new(state_data_vec));
        let state_key: &'static mut Pubkey = Box::leak(Box::new(state_account));
        // msg!("StateMap pre-test account data: {:?}", state_data);
        let state_account_info: AccountInfo<'static> = AccountInfo::new(
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
        let signer_pubkey: &'static mut Pubkey = Box::leak(Box::new(Pubkey::new_unique()));
        let lamportsx: &'static mut u64 = Box::leak(Box::new(0u64));
        let data: &'static mut Vec<u8> = Box::leak(Box::new(vec![]));
        let owner: &'static mut Pubkey = Box::leak(Box::new(Pubkey::default()));
        let signer_account_info: AccountInfo<'static> = AccountInfo::new(
            signer_pubkey,
            true, // is_signer
            false, // is_writable
            lamportsx,
            data,
            owner,
            false,
            0,
        );
        // Create AccountInfo for system_program
        let sys_lamports: &'static mut u64 = Box::leak(Box::new(0u64));
        let sys_data: &'static mut Vec<u8> = Box::leak(Box::new(vec![]));
        let sys_owner: &'static mut Pubkey = Box::leak(Box::new(Pubkey::default()));
        let sys_account_info: AccountInfo<'static> = AccountInfo::new(
            &system_program::ID,
            false, // is_signer
            false, // is_writable
            sys_lamports,
            sys_data,
            sys_owner,
            true,
            0,
        );
        (state_account_info, signer_account_info, sys_account_info)
    }

    fn initialize_anchor(program_id: &'static Pubkey) -> (Account<'static, StateMap>, Signer<'static>, Program<'static, anchor_lang::system_program::System>) {
        //                 state_account_info: &'static AccountInfo<'static>) {
        //                 sys_account_info: &AccountInfo<'static>) {
        // let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let state_account: Pubkey = Pubkey::find_program_address(&[b"state".as_ref()], program_id).0;
        let (state_account_info, irma_admin_account_info, sys_account_info) 
                 = prep_accounts(program_id, state_account);
        // Bind to variables to extend their lifetime
        let state_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(state_account_info));
        let irma_admin_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(irma_admin_account_info));
        let sys_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(sys_account_info));
        let mut accounts: Initialize<'_> = Initialize {
            state: Account::try_from(state_account_static).unwrap(),
            irma_admin: Signer::try_from(irma_admin_account_static).unwrap(),
            system_program: Program::try_from(sys_account_static).unwrap(),
        };
        let ctx: Context<Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            InitializeBumps::default(), // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(ctx);
        assert!(result.is_ok());
        msg!("StateMap account: {:?}", accounts.state);
        return (accounts.state, accounts.irma_admin, accounts.system_program);
    }

    #[test]
    fn test_initialize_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing initialize IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &IRMA_ID;
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: Initialize<'_> = Initialize {
            state: state_account.clone(),
            irma_admin: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let ctx: Context<Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            InitializeBumps::default(), // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(ctx);
        assert!(result.is_ok());
        msg!("StateMap account initialized successfully: {:?}", accounts.state);
    }

    #[test]
    fn test_set_mint_price_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing set IRMA mint price with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &IRMA_ID;
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: IrmaCommon<'_> = IrmaCommon {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        let mut result: std::result::Result<(), Error> = set_mint_price(ctx, "USDT", 1.5);
        assert!(result.is_ok());
        // Re-create ctx for the next call if needed
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = set_mint_price(ctx, "USDC", 1.8);
        assert!(result.is_ok());
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = set_mint_price(ctx, "FDUSD", 1.3);
        assert!(result.is_ok());
        // msg!("Mint price for USDT set successfully: {:?}", accounts.state.mint_price["USDT" as usize]);
        // msg!("Mint price for USDC set successfully: {:?}", accounts.state.mint_price[Stablecoins::USDC as usize]);
        // msg!("Mint price for USDE set successfully: {:?}", accounts.state.mint_price[Stablecoins::FDUSD as usize]);
    }

    #[test]
    fn test_mint_irma_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing mint IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &IRMA_ID;
        // let state_account: Pubkey = Pubkey::find_program_address(&[b"state".as_ref()], program_id).0;
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: IrmaCommon<'_> = IrmaCommon {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-mint IRMA state:");
        msg!("Backing reserves for USDT: {:?}", accounts.state.reserves["USDT"].backing_reserves);
        msg!("Backing reserves for PYUSD: {:?}", accounts.state.reserves["PYUSD"].backing_reserves);
        msg!("Backing reserves for USDG: {:?}", accounts.state.reserves["USDG"].backing_reserves);
        msg!("IRMA in circulation for USDT: {:?}", accounts.state.reserves["USDT"].irma_in_circulation);
        msg!("IRMA in circulation for PYUSD: {:?}", accounts.state.reserves["PYUSD"].irma_in_circulation);
        msg!("IRMA in circulation for USDG: {:?}", accounts.state.reserves["USDG"].irma_in_circulation);
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        let mut result = mint_irma(ctx, "USDT", 100);
        match result {
            Err(e) => {
                msg!("Error minting IRMA for USDT: {:?}", e);
            },
            Ok(_) => {
                msg!("Mint IRMA successful for USDT");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = mint_irma(ctx, "PYUSD", 1000);
        match result {
            Err(e) => {
                msg!("Error minting IRMA for PYUSD: {:?}", e);
            },
            Ok(_) => {
                msg!("Mint IRMA successful for PYUSD");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = mint_irma(ctx, "USDG", 10000);
        match result {
            Err(e) => {
                msg!("Error minting IRMA for USDG: {:?}", e);
            },
            Ok(_) => {
                msg!("Mint IRMA successful for USDG");
            }
        }
        msg!("-------------------------------------------------------------------------");
        msg!("Post-mint IRMA state:");
        msg!("Backing reserves for USDT: {:?}", accounts.state.reserves["USDT"].backing_reserves);
        msg!("Backing reserves for PYUSD: {:?}", accounts.state.reserves["PYUSD"].backing_reserves);
        msg!("Backing reserves for USDG: {:?}", accounts.state.reserves["USDG"].backing_reserves);
        msg!("IRMA in circulation for USDT: {:?}", accounts.state.reserves["USDT"].irma_in_circulation);
        msg!("IRMA in circulation for PYUSD: {:?}", accounts.state.reserves["PYUSD"].irma_in_circulation);
        msg!("IRMA in circulation for USDG: {:?}", accounts.state.reserves["USDG"].irma_in_circulation);
    }


    #[test]
    fn test_redeem_irma_anchor() -> Result<()> {        
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA when mint price is less than redemption price");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &IRMA_ID;
        let (state_account, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let mut accounts: IrmaCommon<'_> = IrmaCommon {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-redeem IRMA state:");
        let state: &mut StateMap = &mut accounts.state;
        let reserves = state.reserves.clone();
        let keys: Vec<String> = reserves.keys().cloned().collect();
        for sc in keys {
            msg!("Backing reserves for {}: {:?}", sc, reserves[&sc].backing_reserves);
            if reserves[&sc].backing_decimals == 0 {
                // require!(*reserve == 0, CustomError::InvalidBacking);
                // require!(*circulation == 1, CustomError::InvalidIrmaAmount);
                continue; // skip non-existent stablecoins
            }
            let mut_reserves = &mut state.reserves;
            let mut_backing = mut_reserves.get_mut(&sc).unwrap();
            let reserve: &mut u64 = &mut mut_backing.backing_reserves;
            let circulation: &mut u64 = &mut mut_backing.irma_in_circulation;
            *reserve = 1000000; // Set a large reserve for testing
            *circulation = 100000; // Set a large IRMA in circulation for testing
        }
        // msg!("Current prices: {:?}", accounts.state.mint_price);
        // msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        // msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        let mut result: std::result::Result<(), Error> = redeem_irma(ctx, "USDC", 10);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDC: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDC");
            }
        }
        // assert!(result.is_ok(), "Redeem IRMA failed for USDC");
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = redeem_irma(ctx, "USDT", 20);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDT: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDT");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = redeem_irma(ctx, "PYUSD", 30);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for PYUSD: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for PYUSD");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = redeem_irma(ctx, "USDG", 40);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDG: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDG");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = redeem_irma(ctx, "FDUSD", 50);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for FDUSD: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for FDUSD");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );

        msg!("Mid-state for USDT before further redemption: {:?}", 
            state_account.reserves["USDT"].backing_reserves);
        // Test for near maximum redemption
        result = redeem_irma(ctx, "USDT", 10_000);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDT: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDT");
            }
        }
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        result = redeem_irma(ctx, "USDS", 10);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDS: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDS");
            }
        }
        msg!("-------------------------------------------------------------------------");
        msg!("Redeem IRMA successful:");
        msg!("Backing reserves for USDT: {:?}", accounts.state.reserves);
        Ok(())
    }

    /// Test cases for when redemption price is less than mint price
    #[test]
    fn test_redeem_irma_normal() -> Result<()> {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA with normal conditions, but with large discrepancies in mint prices");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &IRMA_ID;
        let (state_account, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let mut accounts: IrmaCommon<'_> = IrmaCommon {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        // {
        //     msg!("Pre-redeem IRMA state:");
        //     let state: &mut StateMap = &mut accounts.state;
        //     let reserves = state.reserves.clone();
        //     let keys: Vec<String> = reserves.keys().cloned().collect();
        //     for sc in keys {
        //         msg!("Backing reserves for {}: {:?}", sc, reserves[sc].backing_reserves);
        //         if reserves[sc].backing_decimals == 0 {
        //             // require!(*reserve == 0, CustomError::InvalidBacking);
        //             // require!(*circulation == 1, CustomError::InvalidIrmaAmount);
        //             continue; // skip non-existent stablecoins
        //         }
        //         let mut_backing = state.reserves.get_mut(&sc).unwrap();
        //         let reserve: &mut u64 = &mut mut_backing.backing_reserves;
        //         let circulation: &mut u64 = &mut mut_backing.irma_in_circulation;
        //         let price: &mut f64 = &mut mut_backing.mint_price;
        //         *reserve = 1000000; // Set a large reserve for testing
        //         *circulation = 100000; // Set a large IRMA in circulation for testing
        //         *price = 2.0; // Set a price for testing
        //     }
        // }
        {
            msg!("Pre-redeem IRMA state:");
            let state: &mut StateMap = &mut accounts.state;
            let reserves = state.reserves.clone();
            let keys: Vec<String> = reserves.keys().cloned().collect();
            let mut i: u64 = 0;
            for sc in keys {
                msg!("Backing reserves for {}: {:?}", sc, reserves[&sc].backing_reserves);
                if reserves[&sc].backing_decimals == 0 {
                    require!(reserves[&sc].active == false, CustomError::InvalidBacking);
                    // require!(*circulation == 1, CustomError::InvalidIrmaAmount);
                    continue; // skip non-existent stablecoins
                }
                let mut_backing = state.reserves.get_mut(&sc).unwrap();
                let reserve: &mut u64 = &mut mut_backing.backing_reserves;
                let circulation: &mut u64 = &mut mut_backing.irma_in_circulation;
                let price: &mut f64 = &mut mut_backing.mint_price;
                *reserve = 9_900_000_000; // Set a large reserve for testing
                *circulation = 10_000_000_000; // Set a large IRMA in circulation for testing
                *price = (i as f64 + 1.0) * (i as f64 + 1.0); // Set a price for testing
                i += 1;
            }
        }
        // msg!("Current prices: {:?}", accounts.state.mint_price);
        // msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        // msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps::default(),
        );
        let mut count: u64 = 0;
        // Test for near maximum redemption, multiple times, until it fails.
        // What we expect is that these repeated redemptions will equalize the differences between
        // mint prices and redemptions prices for all stablecoins.
        let mut reslt = redeem_irma(ctx, "FDUSD", 100_000_000_000);
        while reslt.is_ok() {
            ctx = Context::new(
                program_id,
                &mut accounts,
                &[],
                IrmaCommonBumps::default(), // Use default bumps if not needed
            );
            reslt = redeem_irma(ctx, "FDUSD", 100_000_000_000);
            match reslt {
                Err(e) => {
                    msg!("Error redeeming IRMA for USDT: {:?}", e);
                    break; // Exit loop on error
                },
                Ok(_) => {
                    // msg!("Redeem IRMA successful for USDT");
                }
            }

            // Print the current state after every ten redemptions
            if count % 10 == 0 {
                let reserves = &accounts.state.reserves;
                let keys: Vec<String> = reserves.keys().cloned().collect();
                for sc in keys {
                    let backing: u64 = reserves[&sc].backing_reserves;
                    let circulation: u64 = reserves[&sc].irma_in_circulation;
                    let redemption_price: f64 = backing as f64 / circulation as f64;
                    msg!("{}, {:.3}, {}, {}, {:.3}", 
                        sc, 
                        reserves[&sc].mint_price, 
                        backing,
                        circulation,
                        redemption_price);
                }
            }

            count += 1;
        }

        // msg!("-------------------------------------------------------------------------");
        // msg!("Redeem IRMA successful:");
        // msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        // msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        Ok(())
    }
}

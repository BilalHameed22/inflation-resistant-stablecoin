
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
    use irma_program::pricing::CustomError;
    use irma_program::pricing::BACKING_COUNT;
    use irma_program::pricing::{Stablecoins, State, Initialize, IrmaCommon, IrmaCommonBumps, InitializeBumps};
    use irma_program::pricing::{initialize, set_mint_price, mint_irma, redeem_irma, BUMP_VALUE};
    use irma_program::pricing::Stablecoins::EnumCount;
    // use irma_program::pricing::StateWithDiscriminator;

    /// Initialize test state
    /// Always start from a known state to simplify issue analysis.
    fn init_state<'info>() -> (&'info mut State, &'info mut [u8]) {
        let (mut_state, mut_disc) = State::new_mut();

        // let mut state = *mut_state; // this copies the heap memory data itself?
        assert_eq!(mut_state.mint_price.len(), EnumCount as usize);
        assert_eq!(mut_state.backing_reserves.len(), EnumCount as usize);
        assert_eq!(mut_state.irma_in_circulation.len(), EnumCount as usize);
        assert_eq!(mut_state.backing_decimals.len(), EnumCount as usize);
        assert_eq!(mut_state.bump, BUMP_VALUE);
        for i in 0..BACKING_COUNT as usize {
            mut_state.mint_price[i] = 1.0; // Initialize with default price
            mut_state.backing_reserves[i] = 1000; // Initialize with some reserve
            mut_state.irma_in_circulation[i] = 100; // Initialize with some IRMA in circulation
            mut_state.backing_decimals[i] = 6; // Assume 6 decimals for stablecoins
        }
        check_init_condition(mut_state);        
        (mut_state, mut_disc)
    }

    fn check_init_condition(state: &State) {
        assert_eq!(state.mint_price.len(), EnumCount as usize);
        assert_eq!(state.backing_reserves.len(), EnumCount as usize);
        assert_eq!(state.irma_in_circulation.len(), EnumCount as usize);
        assert_eq!(state.backing_decimals.len(), EnumCount as usize);
        assert_eq!(state.bump, BUMP_VALUE);
        for i in BACKING_COUNT as usize..EnumCount as usize {
            assert_eq!(state.mint_price[i], 1.0);
            assert_eq!(state.backing_reserves[i], 0);
            assert_eq!(state.irma_in_circulation[i], 1);
            assert_eq!(state.backing_decimals[i], 0);
        }
        for i in 0..BACKING_COUNT as usize {
            assert_eq!(state.mint_price[i], 1.0);
            assert_eq!(state.backing_reserves[i], 1000);
            assert_eq!(state.irma_in_circulation[i], 100);
            assert_eq!(state.backing_decimals[i], 6);
        }
    }

    #[test]
    fn test_set_state_directly() {
        let (mut_state, _mut_disc) = init_state();
        check_init_condition(mut_state);        
        let quote_token: Stablecoins = Stablecoins::USDT;
        let new_price: f64 = 1.23;
        //state.mint_price[quote_token as usize] = 1.0;
        let mut state: State = *mut_state; // assigning a deference copies all of the data to a new struct
        assert_eq!(state.mint_price[quote_token as usize], 1.0);
        state.mint_price[quote_token as usize] = new_price;
        assert_eq!(state.mint_price[quote_token as usize], new_price);
    }

    #[test]
    fn test_mint_irma_directly() {
        let (mut_state, _mut_disc) = init_state();
        let quote_token = Stablecoins::USDT;
        let amount = 100;
        let price = mut_state.mint_price[quote_token as usize];
        let prev_circulation = mut_state.irma_in_circulation[quote_token as usize];
        let prev_reserve = mut_state.backing_reserves[quote_token as usize];
        // Simulate mint_irma logic
        mut_state.backing_reserves[quote_token as usize] += amount;
        mut_state.irma_in_circulation[quote_token as usize] += (amount as f64 / price).ceil() as u64;
        assert_eq!(mut_state.backing_reserves[quote_token as usize], prev_reserve + amount);
        assert_eq!(mut_state.irma_in_circulation[quote_token as usize], prev_circulation + (amount as f64 / price).ceil() as u64);
    }

    #[test]
    fn test_redeem_irma_simple() {
        let mut state = *init_state().0;
        let quote_token = Stablecoins::USDT;
        let irma_amount = 10;
        let prev_circulation = state.irma_in_circulation[quote_token as usize];
        msg!("Prev IRMA in circulation: {:?}", prev_circulation);
        // Simulate redeem_irma logic (simple case)
        state.irma_in_circulation[quote_token as usize] -= irma_amount;
        assert_eq!(state.irma_in_circulation[quote_token as usize], prev_circulation - irma_amount);
    }

    #[test]
    fn test_reduce_circulations_logic() {
        let mut state = *init_state().0;
        // Manipulate state to create a price difference
        state.mint_price[Stablecoins::USDT as usize] = 2.0;
        state.backing_reserves[Stablecoins::USDT as usize] = 1000;
        state.irma_in_circulation[Stablecoins::USDT as usize] = 100;
        // Should select USDT as first_target
        let quote_token = Stablecoins::USDT;
        let irma_amount = 5;
        let prev_circulation = state.irma_in_circulation[quote_token as usize];
        // Simulate reduce_circulations logic (first_target == quote_token)
        state.irma_in_circulation[quote_token as usize] -= irma_amount;
        assert_eq!(state.irma_in_circulation[quote_token as usize], prev_circulation - irma_amount);
    }

    // Tests that use the pricing functions

    fn set_initial_conditions(state: &mut State, backing: u64, circulation: u64, price: f64) -> Result<State> {
        for i in 0..EnumCount as usize {
            // Only borrow what you need, when you need it
            let backing_decimals = state.backing_decimals[i as usize];
            if backing_decimals == 0 {
                let reserve = state.backing_reserves[i];
                let circulation = state.irma_in_circulation[i];
                require!(reserve == 0, CustomError::InvalidBacking);
                require!(circulation == 1, CustomError::InvalidIrmaAmount);
                continue;
            }
            // Now borrow mutably, but only one at a time
            state.backing_reserves[i] = backing;
            state.irma_in_circulation[i] = circulation;
            if price > 0.0 {
                state.mint_price[i] = price;
            } else {
                state.mint_price[i] = (i as f64 + 1.0) * (i as f64 + 1.0); // Set a price for testing
            }
        }
        Ok(*state)
    }

    fn initialize_anchor<'info>(program_id: &'static Pubkey) -> 
            (AccountLoader<'info, State>, Signer<'info>, Program<'info, anchor_lang::system_program::System>) {

        let mut_state_account: &'static mut Pubkey = Box::leak(Box::new(Pubkey::find_program_address(&[b"state".as_ref()], program_id).0));
        // Create a buffer for State and wrap it in AccountInfo
        // let state_key: &'info mut Pubkey = Box::leak(Box::new(state_account));
        // let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));
        let (_mut_state, mut_disc) = init_state();
        // let state_data: &'info mut [u8] = bytes_of_mut(mut_disc);
        // msg!("State pre-test account data: {:?}", state_data);
        let state_account_info: AccountInfo<'info> = AccountInfo::new(
            mut_state_account, // state_key,
            false, // is_signer
            true,  // is_writable
            Box::leak(Box::new(100000u64)), // lamports
            mut_disc,
            program_id,
            false,
            0,
        );
        // Leak the AccountInfo so it lives for 'info
        let state_account_info: &mut AccountInfo<'info> = Box::leak(Box::new(state_account_info));

        let data_ref = state_account_info.try_borrow_data().unwrap();
        msg!("Data account created: {:?}", &data_ref[..8]);
        // let disc = <State as Discriminator>::DISCRIMINATOR;
        // assert_eq!(&data_ref[..8], disc, "State account discriminator does not match");
        msg!("State account created: {:?}", state_account_info.key);
        msg!("State owner: {:?}", program_id);
        // Use a mock Signer for testing purposes
        let signer_pubkey: &'info mut Pubkey = Box::leak(Box::new(Pubkey::new_unique()));
        // let data: &'info mut [u8] = Box::leak(Box::new([0u8; State::LEN]));
        let owner: &'info mut Pubkey = Box::leak(Box::new(Pubkey::default()));
        let signer_account_info: AccountInfo<'info> = AccountInfo::new(
            signer_pubkey,
            true, // is_signer
            false, // is_writable
            Box::leak(Box::new(100000u64)), // lamports
            &mut [],
            owner,
            false,
            0,
        );
        let signer_account_info: &mut AccountInfo<'info> = Box::leak(Box::new(signer_account_info));

        // Create AccountInfo for system_program
        // let sys_data: &'info mut [u8] = Box::leak(Box::new([0u8; 1024]));
        // let sys_owner: &'info mut Pubkey = Box::leak(Box::new(Pubkey::default()));
        let sys_account_info: AccountInfo<'info> = AccountInfo::new(
            &system_program::ID,
            false, // is_signer
            false, // is_writable
            Box::leak(Box::new(100000u64)), // lamports
            &mut [], // no data
            owner,
            true,
            0,
        );
        let sys_account_info: &AccountInfo<'_> = Box::leak(Box::new(sys_account_info));

        let mut accounts: Initialize<'info> = Initialize {
            state: AccountLoader::try_from(state_account_info).unwrap(),
            irma_admin: Signer::try_from(signer_account_info).unwrap(),
            system_program: Program::try_from(sys_account_info).unwrap(),
        };
        // let _unused = accounts.state.load_init()?;
        let ctx: Context<Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            InitializeBumps { state: 0u8 }, // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(&ctx);
        match result {
            Err(e) => {
                msg!("Error initializing IRMA state: {:?}", e);
            },
            Ok(_) => {
                msg!("IRMA state initialized successfully");
            }
        }
        // assert!(result.is_ok());
        msg!("State account: {:?}", accounts.state);
        return (accounts.state, accounts.irma_admin, accounts.system_program);
    }

    #[test]
    fn test_initialize_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing initialize IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &irma_program::IRMA_ID;
        let (state_loader, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: Initialize<'_> = Initialize {
            state: state_loader.clone(),
            irma_admin: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let ctx: Context<Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            InitializeBumps { state: 0u8 }, // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(&ctx);
        match result {
            Err(e) => {
                msg!("Error initializing IRMA state: {:?}", e);
            },
            Ok(_) => {
                msg!("IRMA state initialized successfully");
            }
        }
        // assert!(result.is_ok());
        msg!("State loader initialized successfully: {:?}", accounts.state);
    }

    #[test]
    fn test_set_mint_price_anchor<'info>() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing set IRMA mint price with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &irma_program::IRMA_ID;
        let (state_loader, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: IrmaCommon<'info> = IrmaCommon {
            state: state_loader.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        let mut result: std::result::Result<(), Error> = set_mint_price(&ctx, Stablecoins::USDT, 1.5);
        assert!(result.is_ok());
        // Re-create ctx for the next call if needed
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = set_mint_price(&ctx, Stablecoins::USDC, 1.8);
        assert!(result.is_ok());
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = set_mint_price(&ctx, Stablecoins::FDUSD, 1.3);
        assert!(result.is_ok());
        // let state: State = *accounts.state.load().unwrap();
        // msg!("Mint price for USDT set successfully: {:?}", state.mint_price[Stablecoins::USDT as usize]);
        // msg!("Mint price for USDC set successfully: {:?}", state.mint_price[Stablecoins::USDC as usize]);
        // msg!("Mint price for USDE set successfully: {:?}", state.mint_price[Stablecoins::FDUSD as usize]);
    }

    #[test]
    fn test_mint_irma_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing mint IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &irma_program::IRMA_ID;
        // let state_loader: Pubkey = Pubkey::find_program_address(&[b"state".as_ref()], program_id).0;
        let (state_loader, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: IrmaCommon<'_> = IrmaCommon {
            state: state_loader.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        // let state: State = *accounts.state.load().unwrap();
        // msg!("Pre-mint IRMA state:");
        // msg!("Backing reserves for USDT: {:?}", state.backing_reserves[Stablecoins::USDT as usize]);
        // msg!("Backing reserves for PYUSD: {:?}", state.backing_reserves[Stablecoins::PYUSD as usize]);
        // msg!("Backing reserves for USDG: {:?}", state.backing_reserves[Stablecoins::USDG as usize]);
        // msg!("IRMA in circulation for USDT: {:?}", state.irma_in_circulation[Stablecoins::USDT as usize]);
        // msg!("IRMA in circulation for PYUSD: {:?}", state.irma_in_circulation[Stablecoins::PYUSD as usize]);
        // msg!("IRMA in circulation for USDG: {:?}", state.irma_in_circulation[Stablecoins::USDG as usize]);
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            &mut accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        let mut result = mint_irma(&ctx, Stablecoins::USDT, 100);
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
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = mint_irma(&ctx, Stablecoins::PYUSD, 1000);
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
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = mint_irma(&ctx, Stablecoins::USDG, 10000);
        match result {
            Err(e) => {
                msg!("Error minting IRMA for USDG: {:?}", e);
            },
            Ok(_) => {
                msg!("Mint IRMA successful for USDG");
            }
        }
        let state: State = *accounts.state.load().unwrap();
        msg!("-------------------------------------------------------------------------");
        msg!("Post-mint IRMA state:");
        msg!("Backing reserves for USDT: {:?}", state.backing_reserves[Stablecoins::USDT as usize]);
        msg!("Backing reserves for PYUSD: {:?}", state.backing_reserves[Stablecoins::PYUSD as usize]);
        msg!("Backing reserves for USDG: {:?}", state.backing_reserves[Stablecoins::USDG as usize]);
        msg!("IRMA in circulation for USDT: {:?}", state.irma_in_circulation[Stablecoins::USDT as usize]);
        msg!("IRMA in circulation for PYUSD: {:?}", state.irma_in_circulation[Stablecoins::PYUSD as usize]);
        msg!("IRMA in circulation for USDG: {:?}", state.irma_in_circulation[Stablecoins::USDG as usize]);
    }


    #[test]
    fn test_redeem_irma_anchor() -> Result<()> {        
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA when mint price is less than backing price");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &irma_program::IRMA_ID;
        let (state_loader, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let mut accounts: IrmaCommon = IrmaCommon {
            state: state_loader.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let state = accounts.state.clone();
        let state: &mut State = &mut state.load_mut().unwrap();
        // let state = Box::leak(Box::new(state.load_mut().unwrap()));

        let mut_accounts: &mut IrmaCommon<'static> = &mut accounts; // Box::leak(Box::new(accounts));
        msg!("Pre-redeem IRMA state:");
        let result = set_initial_conditions(state, 1000000, 100000, 1.0);
        match result {
            Err(e) => {
                msg!("Error initializing state: {:?}", e);
            },
            Ok(state) => {
                msg!("State initialized successfully");
                msg!("Current prices: {:?}", state.mint_price);
                msg!("Backing reserves: {:?}", state.backing_reserves);
                msg!("IRMA in circulation: {:?}", state.irma_in_circulation);
            }
        }
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        let mut result: std::result::Result<(), Error> = redeem_irma(&ctx, Stablecoins::USDC, 10);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = redeem_irma(&ctx, Stablecoins::USDT, 20);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = redeem_irma(&ctx, Stablecoins::PYUSD, 30);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = redeem_irma(&ctx, Stablecoins::USDG, 40);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = redeem_irma(&ctx, Stablecoins::FDUSD, 50);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );

        msg!("Mid-state for USDT before further redemption: {:?}", 
            state.backing_reserves[Stablecoins::USDT as usize]);
        // Test for near maximum redemption
        result = redeem_irma(&ctx, Stablecoins::USDT, 10_000);
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
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        result = redeem_irma(&ctx, Stablecoins::USDS, 10);
        match result {
            Err(e) => {
                msg!("Error redeeming IRMA for USDS: {:?}", e);
            },
            Ok(_) => {
                msg!("Redeem IRMA successful for USDS");
            }
        }
        // let state: State = *mut_accounts.state.load().unwrap();
        msg!("-------------------------------------------------------------------------");
        msg!("Redeem IRMA successful:");
        msg!("Backing reserves for USDT: {:?}", state.backing_reserves);
        msg!("IRMA in circulation for USDT: {:?}", state.irma_in_circulation);

        Ok(())
    }

    #[test]
    fn test_redeem_irma_normal() -> Result<()> {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = &irma_program::IRMA_ID;
        let (state_loader, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let accounts: IrmaCommon<'static> = IrmaCommon {
            state: state_loader.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-redeem IRMA state:");
        let state = accounts.state.clone();
        let state: &mut State = &mut state.load_mut().unwrap();
        let mut_accounts: &mut IrmaCommon<'static> = Box::leak(Box::new(accounts));
        let result = set_initial_conditions(state, 9_900_000_000, 10_000_000_000, 0.0);
        match result {
            Err(e) => {
                msg!("Error initializing state: {:?}", e);
            },
            Ok(state) => {
                msg!("State initialized successfully");
                msg!("Current prices: {:?}", state.mint_price);
                msg!("Backing reserves: {:?}", state.backing_reserves);
                msg!("IRMA in circulation: {:?}", state.irma_in_circulation);
            }
        }
        let mut ctx: Context<IrmaCommon> = Context::new(
            program_id,
            mut_accounts,
            &[],
            IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
        );
        // Test for near maximum redemption, multiple times, until it fails.
        // What we expect is that these repeated redemptions will equalize the differences between
        // mint prices and redemptions prices for all stablecoins.
        let mut reslt = redeem_irma(&ctx, Stablecoins::USDT, 100_000);
        while reslt.is_ok() {
            ctx = Context::new(
                program_id,
                mut_accounts,
                &[],
                IrmaCommonBumps { state: 0u8 }, // Use default bumps if not needed
            );
            reslt = redeem_irma(&ctx, Stablecoins::USDT, 100_000);
            match reslt {
                Err(e) => {
                    msg!("Error redeeming IRMA for USDT: {:?}", e);
                    break; // Exit loop on error
                },
                Ok(_) => {
                    msg!("Redeem IRMA successful for USDT");
                }
            }
            reslt = redeem_irma(&ctx, Stablecoins::PYUSD, 1_000);
            match reslt {
                Err(e) => {
                    msg!("Error redeeming IRMA for PYUSD: {:?}", e);
                    break; // Exit loop on error
                },
                Ok(_) => {
                    msg!("Redeem IRMA successful for PYUSD");
                }
            }
            let state: State = *mut_accounts.state.load().unwrap();
            for i in 0..BACKING_COUNT as usize {
                msg!("Backing reserves for {:?}", state.backing_reserves[i]);
                msg!("IRMA in circulation for {:?}", state.irma_in_circulation[i]);
                msg!("Mint price for {:?}", state.mint_price[i]);
            }
        }
        msg!("-------------------------------------------------------------------------");
        msg!("Redeem IRMA successful:");
        msg!("Backing reserves: {:?}", state.backing_reserves);
        msg!("IRMA in circulation: {:?}", state.irma_in_circulation);
        Ok(())
    }
}

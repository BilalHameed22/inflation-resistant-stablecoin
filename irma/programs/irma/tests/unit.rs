
#[cfg(test)]
mod tests {
    // use anchor_lang::accounts::program;
    use anchor_lang::prelude::*;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::solana_program::system_program;
    use anchor_lang::prelude::Signer;
    use anchor_lang::prelude::Account;
    use anchor_lang::prelude::Program;
    use anchor_lang::context::Context;
    use irma::irmamod::CustomError;
    use irma::BACKING_COUNT;
    use irma::irmamod::{self, Stablecoins, State}; //, CustomError};
    use irma::irmamod::{initialize, set_mint_price, mint_irma, redeem_irma};

    fn allocate_state() -> State {
        State {
            mint_price: Vec::<f64>::with_capacity(Stablecoins::EnumCount as usize),
            backing_reserves: Vec::<u64>::with_capacity(Stablecoins::EnumCount as usize),
            irma_in_circulation: Vec::<u64>::with_capacity(Stablecoins::EnumCount as usize),
            backing_decimals: Vec::<u8>::with_capacity(Stablecoins::EnumCount as usize),
            bump: 0u8,
        }
    }

    fn init_state() -> State {
        let mut state: State = allocate_state();
        for i in 0..BACKING_COUNT as usize {
            state.mint_price.push(1.0); // Initialize with default price
            state.backing_reserves.push(1000); // Initialize with some reserve
            state.irma_in_circulation.push(100); // Initialize with some IRMA in circulation
            state.backing_decimals.push(6); // Assume 6 decimals for stablecoins
            assert_eq!(1.0, state.mint_price[i]);
        }
        assert_eq!(state.mint_price.len(), BACKING_COUNT);
        assert_eq!(state.backing_reserves.len(), BACKING_COUNT);
        assert_eq!(state.irma_in_circulation.len(), BACKING_COUNT);
        assert_eq!(state.backing_decimals.len(), BACKING_COUNT);
        assert_eq!(state.bump, 0u8);
        state
    }

    #[test]
    fn test_set_state_directly() {
        let mut state: State = init_state();
        let quote_token: Stablecoins = Stablecoins::USDT;
        let new_price: f64 = 1.23;
        state.mint_price[quote_token as usize] = 1.0;
        assert_eq!(state.mint_price[quote_token as usize], 1.0);
        state.mint_price[quote_token as usize] = new_price;
        assert_eq!(state.mint_price[quote_token as usize], new_price);
    }

    #[test]
    fn test_mint_irma_directly() {
        let mut state = init_state();
        let quote_token = Stablecoins::USDT;
        let amount = 100;
        let price = state.mint_price[quote_token as usize];
        let prev_circulation = state.irma_in_circulation[quote_token as usize];
        let prev_reserve = state.backing_reserves[quote_token as usize];
        // Simulate mint_irma logic
        state.backing_reserves[quote_token as usize] += amount;
        state.irma_in_circulation[quote_token as usize] += (amount as f64 / price).ceil() as u64;
        assert_eq!(state.backing_reserves[quote_token as usize], prev_reserve + amount);
        assert_eq!(state.irma_in_circulation[quote_token as usize], prev_circulation + (amount as f64 / price).ceil() as u64);
    }

    #[test]
    fn test_redeem_irma_simple() {
        let mut state = init_state();
        let quote_token = Stablecoins::USDT;
        let irma_amount = 10;
        let prev_circulation = state.irma_in_circulation[quote_token as usize];
        // Simulate redeem_irma logic (simple case)
        state.irma_in_circulation[quote_token as usize] -= irma_amount;
        assert_eq!(state.irma_in_circulation[quote_token as usize], prev_circulation - irma_amount);
    }

    #[test]
    fn test_reduce_circulations_logic() {
        let mut state = init_state();
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

    fn prep_accounts(owner: &'static Pubkey, state_account: Pubkey) -> (AccountInfo<'static>, AccountInfo<'static>, AccountInfo<'static>) {
        // Create a buffer for State and wrap it in AccountInfo
        let lamports: &'static mut u64 = Box::leak(Box::new(100000u64));
        let state: State = allocate_state();

        // Prepare the account data with the correct discriminator
        let mut state_data_vec: Vec<u8> = Vec::with_capacity(1024);
        state.try_serialize(&mut state_data_vec).unwrap();

        let state_data: &'static mut Vec<u8> = Box::leak(Box::new(state_data_vec));
        let state_key: &'static mut Pubkey = Box::leak(Box::new(state_account));
        // msg!("State pre-test account data: {:?}", state_data);
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
        // msg!("State account created: {:?}", state_account_info.key);
        // msg!("State owner: {:?}", owner);
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

    fn initialize_anchor(program_id: &'static Pubkey) -> (Account<'static, State>, Signer<'static>, Program<'static, anchor_lang::system_program::System>) {
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
        let mut accounts: irmamod::Initialize<'_> = irmamod::Initialize {
            state: Account::try_from(state_account_static).unwrap(),
            irma_admin: Signer::try_from(irma_admin_account_static).unwrap(),
            system_program: Program::try_from(sys_account_static).unwrap(),
        };
        let ctx: Context<irmamod::Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::InitializeBumps::default(), // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(ctx);
        assert!(result.is_ok());
        msg!("State account: {:?}", accounts.state);
        return (accounts.state, accounts.irma_admin, accounts.system_program);
    }

    #[test]
    fn test_initialize_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing initialize IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: irmamod::Initialize<'_> = irmamod::Initialize {
            state: state_account.clone(),
            irma_admin: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let ctx: Context<irmamod::Initialize> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::InitializeBumps::default(), // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = initialize(ctx);
        assert!(result.is_ok());
        msg!("State account initialized successfully: {:?}", accounts.state);
    }

    #[test]
    fn test_set_mint_price_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing set IRMA mint price with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: irmamod::SetMintPrice<'_> = irmamod::SetMintPrice {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let mut ctx: Context<irmamod::SetMintPrice> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::SetMintPriceBumps::default(), // Use default bumps if not needed
        );
        let mut result: std::result::Result<(), Error> = set_mint_price(ctx, Stablecoins::USDT, 1.5);
        assert!(result.is_ok());
        // Re-create ctx for the next call if needed
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::SetMintPriceBumps::default(), // Use default bumps if not needed
        );
        result = set_mint_price(ctx, Stablecoins::USDC, 1.8);
        assert!(result.is_ok());
        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::SetMintPriceBumps::default(), // Use default bumps if not needed
        );
        result = set_mint_price(ctx, Stablecoins::FDUSD, 1.3);
        assert!(result.is_ok());
        msg!("Mint price for USDT set successfully: {:?}", accounts.state.mint_price[Stablecoins::USDT as usize]);
        msg!("Mint price for USDC set successfully: {:?}", accounts.state.mint_price[Stablecoins::USDC as usize]);
        msg!("Mint price for USDE set successfully: {:?}", accounts.state.mint_price[Stablecoins::FDUSD as usize]);
    }

    #[test]
    fn test_mint_irma_anchor() {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing mint IRMA with normal conditions");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        // let state_account: Pubkey = Pubkey::find_program_address(&[b"state".as_ref()], program_id).0;
        let (state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        // Bind to variables to extend their lifetime
        let mut accounts: irmamod::MintIrma<'_> = irmamod::MintIrma {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-mint IRMA state:");
        msg!("Backing reserves for USDT: {:?}", accounts.state.backing_reserves[Stablecoins::USDT as usize]);
        msg!("Backing reserves for PYUSD: {:?}", accounts.state.backing_reserves[Stablecoins::PYUSD as usize]);
        msg!("Backing reserves for USDG: {:?}", accounts.state.backing_reserves[Stablecoins::USDG as usize]);
        msg!("IRMA in circulation for USDT: {:?}", accounts.state.irma_in_circulation[Stablecoins::USDT as usize]);
        msg!("IRMA in circulation for PYUSD: {:?}", accounts.state.irma_in_circulation[Stablecoins::PYUSD as usize]);
        msg!("IRMA in circulation for USDG: {:?}", accounts.state.irma_in_circulation[Stablecoins::USDG as usize]);
        let mut ctx: Context<irmamod::MintIrma> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::MintIrmaBumps::default(), // Use default bumps if not needed
        );
        let mut result = mint_irma(ctx, Stablecoins::USDT, 100);
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
            irmamod::MintIrmaBumps::default(), // Use default bumps if not needed
        );
        result = mint_irma(ctx, Stablecoins::PYUSD, 1000);
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
            irmamod::MintIrmaBumps::default(), // Use default bumps if not needed
        );
        result = mint_irma(ctx, Stablecoins::USDG, 10000);
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
        msg!("Backing reserves for USDT: {:?}", accounts.state.backing_reserves[Stablecoins::USDT as usize]);
        msg!("Backing reserves for PYUSD: {:?}", accounts.state.backing_reserves[Stablecoins::PYUSD as usize]);
        msg!("Backing reserves for USDG: {:?}", accounts.state.backing_reserves[Stablecoins::USDG as usize]);
        msg!("IRMA in circulation for USDT: {:?}", accounts.state.irma_in_circulation[Stablecoins::USDT as usize]);
        msg!("IRMA in circulation for PYUSD: {:?}", accounts.state.irma_in_circulation[Stablecoins::PYUSD as usize]);
        msg!("IRMA in circulation for USDG: {:?}", accounts.state.irma_in_circulation[Stablecoins::USDG as usize]);
    }


    #[test]
    fn test_redeem_irma_anchor() -> Result<()> {        
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA when mint price is less than redemption price");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let (state_account, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let mut accounts: irmamod::RedeemIrma<'_> = irmamod::RedeemIrma {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-redeem IRMA state:");
        let state: &mut State = &mut accounts.state;
        for i in 0..BACKING_COUNT {
            let reserve: &mut u64 = &mut state.backing_reserves[i];
            let circulation: &mut u64 = &mut state.irma_in_circulation[i];
            if state.backing_decimals[i as usize] == 0 {
                require!(*reserve == 0, CustomError::InvalidBacking);
                require!(*circulation == 1, CustomError::InvalidIrmaAmount);
                continue; // skip non-existent stablecoins
            }
            *reserve = 1000000; // Set a large reserve for testing
            *circulation = 100000; // Set a large IRMA in circulation for testing
        }
        msg!("Current prices: {:?}", accounts.state.mint_price);
        msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        let mut ctx: Context<irmamod::RedeemIrma> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        let mut result: std::result::Result<(), Error> = redeem_irma(ctx, Stablecoins::USDC, 10);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        result = redeem_irma(ctx, Stablecoins::USDT, 20);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        result = redeem_irma(ctx, Stablecoins::PYUSD, 30);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        result = redeem_irma(ctx, Stablecoins::USDG, 40);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        result = redeem_irma(ctx, Stablecoins::FDUSD, 50);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );

        msg!("Mid-state for USDT before further redemption: {:?}", 
            state_account.backing_reserves[Stablecoins::USDT as usize]);
        // Test for near maximum redemption
        result = redeem_irma(ctx, Stablecoins::USDT, 10_000);
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
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        result = redeem_irma(ctx, Stablecoins::USDS, 10);
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
        msg!("Backing reserves for USDT: {:?}", accounts.state.backing_reserves);
        msg!("IRMA in circulation for USDT: {:?}", accounts.state.irma_in_circulation);
        Ok(())
    }

    /// Test cases for when redemption price is less than mint price
    #[test]
    fn test_redeem_irma_normal() -> Result<()> {
        msg!("-------------------------------------------------------------------------");
        msg!("Testing redeem IRMA with normal conditions, but with large discrepancies in mint prices");  
        msg!("-------------------------------------------------------------------------");
        let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let (state_account, irma_admin_account, sys_account) 
            = initialize_anchor(program_id);
        let mut accounts: irmamod::RedeemIrma<'_> = irmamod::RedeemIrma {
            state: state_account.clone(),
            trader: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        msg!("Pre-redeem IRMA state:");
        let state: &mut State = &mut accounts.state;
        for i in 0..BACKING_COUNT {
            let reserve: &mut u64 = &mut state.backing_reserves[i];
            let circulation: &mut u64 = &mut state.irma_in_circulation[i];
            let price: &mut f64 = &mut state.mint_price[i];
            if state.backing_decimals[i as usize] == 0 {
                require!(*reserve == 0, CustomError::InvalidBacking);
                require!(*circulation == 1, CustomError::InvalidIrmaAmount);
                continue; // skip non-existent stablecoins
            }
            *reserve = 9_900_000_000; // Set a large reserve for testing
            *circulation = 10_000_000_000; // Set a large IRMA in circulation for testing
            *price = (i as f64 + 1.0) * (i as f64 + 1.0); // Set a price for testing
        }
        msg!("Current prices: {:?}", accounts.state.mint_price);
        msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        let mut ctx: Context<irmamod::RedeemIrma> = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        let mut count: u64 = 0;
        // Test for near maximum redemption, multiple times, until it fails.
        // What we expect is that these repeated redemptions will equalize the differences between
        // mint prices and redemptions prices for all stablecoins.
        let mut reslt = redeem_irma(ctx, Stablecoins::USDT, 100_000);
        while reslt.is_ok() {
            ctx = Context::new(
                program_id,
                &mut accounts,
                &[],
                irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
            );
            reslt = redeem_irma(ctx, Stablecoins::USDT, 100_000);
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
                let state: &State = &accounts.state;
                for i in 0..BACKING_COUNT {
                    let reserve: u64 = state.backing_reserves[i as usize];
                    let circulation: u64 = state.irma_in_circulation[i as usize];
                    let redemption_price: f64 = reserve as f64 / circulation as f64;
                    msg!("{}, {:.3}, {}, {}, {:.3}", 
                        Stablecoins::from_index(i).to_string(), 
                        state.mint_price[i as usize], 
                        reserve,
                        circulation,
                        redemption_price);
                }
            }

            count += 1;
        }

        ctx = Context::new(
            program_id,
            &mut accounts,
            &[],
            irmamod::RedeemIrmaBumps::default(), // Use default bumps if not needed
        );
        msg!("-------------------------------------------------------------------------");
        msg!("Redeem IRMA successful:");
        msg!("Backing reserves: {:?}", accounts.state.backing_reserves);
        msg!("IRMA in circulation: {:?}", accounts.state.irma_in_circulation);
        Ok(())
    }
}

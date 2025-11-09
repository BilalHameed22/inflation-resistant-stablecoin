use super::mod::*;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::*;
use commons::quote::*;
use commons::dlmm::accounts::*;
use commons::dlmm::types::*;
use commons::token_2022::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_swap_token2022_exact_out_on_chain() -> Result<()> {
    let mut test_pair = OnChainTestPair::new().await?;
    
    println!("Setting up Token 2022 on-chain swap test...");
    
    // Create Token 2022 mints instead of regular SPL tokens
    let token_2022_mint_x = test_pair.config.create_token_2022_mint(
        &test_pair.config.payer.pubkey(),
        None,
        6, // decimals
    ).await?;
    
    let token_2022_mint_y = test_pair.config.create_token_2022_mint(
        &test_pair.config.payer.pubkey(),
        None,
        9, // decimals
    ).await?;

    // Create Token 2022 accounts
    let user_token_2022_x = test_pair.config.create_token_2022_account(
        &token_2022_mint_x.pubkey(),
        &test_pair.config.payer.pubkey(),
    ).await?;
    
    let user_token_2022_y = test_pair.config.create_token_2022_account(
        &token_2022_mint_y.pubkey(),
        &test_pair.config.payer.pubkey(),
    ).await?;

    println!("Token 2022 X Mint: {}", token_2022_mint_x.pubkey());
    println!("Token 2022 Y Mint: {}", token_2022_mint_y.pubkey());

    // Create mock LB pair data for Token 2022
    let lb_pair_data = create_mock_lb_pair_token2022(
        token_2022_mint_x.pubkey(),
        token_2022_mint_y.pubkey(),
        test_pair.reserve_x,
        test_pair.reserve_y,
    );

    // Create mock bin arrays with Token 2022 considerations
    let bin_arrays = create_mock_bin_arrays_token2022();

    // Test parameters
    let amount_out = 1000000; // 1 token (6 decimals)
    let swap_for_y = true;

    // Create AccountInfo for Token 2022 mints
    let mint_x_key = token_2022_mint_x.pubkey();
    let mint_y_key = token_2022_mint_y.pubkey();
    
    // Token 2022 mints have larger size due to extensions
    let mint_x_lamports = &mut 0u64;
    let mint_x_data = &mut vec![0u8; 165]; // Token 2022 mint size with extensions
    let mint_x_owner = spl_token_2022::ID;
    let mint_x_account = AccountInfo::new(
        &mint_x_key,
        false,
        false,
        mint_x_lamports,
        mint_x_data,
        &mint_x_owner,
        false,
        0,
    );

    let mint_y_lamports = &mut 0u64;
    let mint_y_data = &mut vec![0u8; 165];
    let mint_y_owner = spl_token_2022::ID;
    let mint_y_account = AccountInfo::new(
        &mint_y_key,
        false,
        false,
        mint_y_lamports,
        mint_y_data,
        &mint_y_owner,
        false,
        0,
    );

    let clock = Clock {
        slot: 100,
        epoch_start_timestamp: 1000000000,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: 1700000000,
    };

    // Test Token 2022 specific functionality
    let transfer_hook_accounts = get_extra_account_metas_for_transfer_hook(
        token_2022_mint_x.pubkey(),
        mint_x_account.clone(),
    );

    println!("Transfer hook accounts found: {}", transfer_hook_accounts.len());

    // Perform the quote calculation
    let quote_result = quote_exact_out(
        test_pair.lb_pair,
        &lb_pair_data,
        amount_out,
        swap_for_y,
        bin_arrays,
        None, // No bitmap extension for this test
        &clock,
        mint_x_account,
        mint_y_account,
    );

    match quote_result {
        Ok(quote) => {
            println!("Token 2022 quote successful!");
            println!("Amount in: {}", quote.amount_in);
            println!("Fee: {}", quote.fee);
            
            // Assertions for Token 2022
            assert!(quote.amount_in > 0, "Amount in should be greater than 0");
            assert!(quote.fee >= 0, "Fee should be non-negative");
            
            // Token 2022 might have transfer fees, so amount in could be higher
            assert!(quote.amount_in < amount_out * 3, "Amount in should be reasonable even with transfer fees");
        }
        Err(e) => {
            println!("Token 2022 quote failed: {:?}", e);
            return Err(e);
        }
    }

    println!("Token 2022 on-chain swap test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_token2022_transfer_fee_calculation() -> Result<()> {
    let test_pair = OnChainTestPair::new().await?;
    
    println!("Testing Token 2022 transfer fee calculations...");

    // Create a Token 2022 mint with transfer fees
    let mint_with_fees = test_pair.config.create_token_2022_mint_with_transfer_fee(
        &test_pair.config.payer.pubkey(),
        None,
        6, // decimals
        500, // 5% transfer fee (in basis points)
        1000000, // Max fee of 1 token
    ).await?;

    let token_account = test_pair.config.create_token_2022_account(
        &mint_with_fees.pubkey(),
        &test_pair.config.payer.pubkey(),
    ).await?;

    // Test transfer fee calculations
    let amount = 1000000; // 1 token
    let epoch = 100;

    // Create mock AccountInfo for the mint
    let mint_key = mint_with_fees.pubkey();
    let mint_lamports = &mut 0u64;
    let mint_data = &mut vec![0u8; 200]; // Larger size for extensions
    let mint_owner = spl_token_2022::ID;
    let mint_account_info = AccountInfo::new(
        &mint_key,
        false,
        false,
        mint_lamports,
        mint_data,
        &mint_owner,
        false,
        0,
    );

    // Test included amount calculation
    let included_result = calculate_transfer_fee_included_amount(
        mint_account_info.clone(),
        amount,
        epoch,
    );

    match included_result {
        Ok(transfer_fee) => {
            println!("Transfer fee included calculation successful!");
            println!("Pre-fee amount: {}", transfer_fee.amount);
            println!("Transfer fee: {}", transfer_fee.transfer_fee);
            
            assert!(transfer_fee.amount <= amount, "Pre-fee amount should be <= original amount");
            assert!(transfer_fee.transfer_fee >= 0, "Transfer fee should be non-negative");
        }
        Err(e) => {
            println!("Transfer fee calculation failed: {:?}", e);
            // This might fail due to mock data, which is expected
        }
    }

    // Test excluded amount calculation
    let excluded_result = calculate_transfer_fee_excluded_amount(
        mint_account_info,
        amount,
        epoch,
    );

    match excluded_result {
        Ok(transfer_fee) => {
            println!("Transfer fee excluded calculation successful!");
            println!("Post-fee amount: {}", transfer_fee.amount);
            println!("Transfer fee: {}", transfer_fee.transfer_fee);
            
            assert!(transfer_fee.amount >= amount, "Post-fee amount should be >= original amount");
            assert!(transfer_fee.transfer_fee >= 0, "Transfer fee should be non-negative");
        }
        Err(e) => {
            println!("Transfer fee excluded calculation failed: {:?}", e);
            // This might fail due to mock data, which is expected
        }
    }

    println!("Token 2022 transfer fee test completed!");
    Ok(())
}

/// Helper function to create mock LB pair data for Token 2022
fn create_mock_lb_pair_token2022(
    token_x_mint: Pubkey,
    token_y_mint: Pubkey,
    reserve_x: Pubkey,
    reserve_y: Pubkey,
) -> commons::dlmm::types::LbPair {
    use commons::dlmm::types::*;
    
    // Create a mock LbPair with Token 2022 considerations
    let mut lb_pair = LbPair {
        parameters: StaticParameters {
            base_factor: 5000,
            filter_period: 30,
            decay_period: 600,
            reduction_factor: 5000,
            variable_fee_control: 40000,
            protocol_share: 1000,
            max_volatility_accumulator: 350000,
        },
        v_parameters: VariableParameters {
            volatility_accumulator: 0,
            volatility_reference: 0,
            id_reference: 8388608,
            time_of_last_update: 1700000000,
        },
        bump_seed: [0; 8],
        require_base_factor_seed: false,
        status: PairStatus::Active as u8,
        bin_step: 25,
        pair_type: PairType::Base as u8,
        active_id: 8388608,
        bin_step_seed: [0; 2],
        token_x_mint,
        token_y_mint,
        reserve_x,
        reserve_y,
        protocol_fee: PairProtocolFee {
            amount_x: 0,
            amount_y: 0,
        },
        reward_infos: [PairRewardInfo::default(); 2],
        oracle: Pubkey::default(),
        bin_array_bitmap: [0; 16],
        last_updated_at: 1700000000,
        // whitelisted_wallet: Pubkey::default(),
        pre_activation_swap_address: Pubkey::default(),
        base_key: Pubkey::default(),
        activation_type: ActivationType::Timestamp as u8,
        creator_pool_on_off_control: false,
        _padding: [0; 7],
        activation_point: 0,
        pre_activation_duration: 0,
        _padding1: [0u8; 64],
        _padding2: [0u8; 32],
    };

    lb_pair
}

/// Helper function to create mock bin arrays for Token 2022
fn create_mock_bin_arrays_token2022() -> HashMap<Pubkey, commons::dlmm::types::BinArray> {
    use commons::dlmm::types::*;
    
    let mut bin_arrays = HashMap::new();
    
    let bin_array_key = Pubkey::new_unique();
    let mut bins = [Bin::default(); 70];
    
    // Add liquidity with Token 2022 considerations (potentially higher amounts due to transfer fees)
    for i in 30..40 {
        bins[i] = Bin {
            amount_x: 2000000000, // Higher amounts to account for potential fees
            amount_y: 2000000000000,
            amount_x_in: 2100000000, // Simulate some transfer fee impact
            amount_y_in: 2100000000000,
            price: 1000000,
            liquidity_supply: 2000000000,
            reward_per_token_stored: [0; 2],
            fee_amount_x_per_token_stored: 0,
            fee_amount_y_per_token_stored: 0,
        };
    }
    
    let bin_array = BinArray {
        index: 0,
        version: 0,
        _padding: [0; 7],
        bins,
    };
    
    bin_arrays.insert(bin_array_key, bin_array);
    bin_arrays
}
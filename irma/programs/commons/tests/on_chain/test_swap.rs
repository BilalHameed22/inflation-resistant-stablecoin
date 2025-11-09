use super::mod::*;
use anchor_lang::prelude::*;
use commons::quote::*;
use commons::dlmm::accounts::*;
use commons::dlmm::types::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_swap_exact_out_on_chain() -> Result<()> {
    let test_pair = OnChainTestPair::new().await?;
    
    println!("Setting up on-chain swap test...");
    println!("Token X Mint: {}", test_pair.token_x_mint.pubkey());
    println!("Token Y Mint: {}", test_pair.token_y_mint.pubkey());
    println!("LB Pair: {}", test_pair.lb_pair);

    // Create mock LB pair data for testing
    let lb_pair_data = create_mock_lb_pair(
        test_pair.token_x_mint.pubkey(),
        test_pair.token_y_mint.pubkey(),
        test_pair.reserve_x,
        test_pair.reserve_y,
    );

    // Create mock bin arrays
    let bin_arrays = create_mock_bin_arrays();

    // Test parameters
    let amount_out = 1000000; // 1 token (6 decimals)
    let swap_for_y = true;

    // Create mock account info for the mints
    let mint_x_key = test_pair.token_x_mint.pubkey();
    let mint_y_key = test_pair.token_y_mint.pubkey();
    
    // Create minimal AccountInfo for testing
    let mint_x_lamports = &mut 0u64;
    let mint_x_data = &mut vec![0u8; 82]; // Standard mint size
    let mint_x_owner = spl_token::ID;
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
    let mint_y_data = &mut vec![0u8; 82];
    let mint_y_owner = spl_token::ID;
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

    // Create mock clock
    let clock = Clock {
        slot: 100,
        epoch_start_timestamp: 1000000000,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: 1700000000,
    };

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
            println!("Quote successful!");
            println!("Amount in: {}", quote.amount_in);
            println!("Fee: {}", quote.fee);
            
            // Basic assertions
            assert!(quote.amount_in > 0, "Amount in should be greater than 0");
            assert!(quote.fee >= 0, "Fee should be non-negative");
            
            // The amount in should be reasonable (not too high)
            assert!(quote.amount_in < amount_out * 2, "Amount in should be reasonable");
        }
        Err(e) => {
            println!("Quote failed: {:?}", e);
            return Err(e);
        }
    }

    println!("On-chain swap test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_swap_exact_in_on_chain() -> Result<()> {
    let test_pair = OnChainTestPair::new().await?;
    
    println!("Setting up on-chain swap exact in test...");

    // Create mock LB pair data
    let lb_pair_data = create_mock_lb_pair(
        test_pair.token_x_mint.pubkey(),
        test_pair.token_y_mint.pubkey(),
        test_pair.reserve_x,
        test_pair.reserve_y,
    );

    // Create mock bin arrays
    let bin_arrays = create_mock_bin_arrays();

    // Test parameters
    let amount_in = 1000000; // 1 token (6 decimals)
    let swap_for_y = false; // Swap X for Y

    // Create minimal AccountInfo for testing
    let mint_x_key = test_pair.token_x_mint.pubkey();
    let mint_y_key = test_pair.token_y_mint.pubkey();
    
    let mint_x_lamports = &mut 0u64;
    let mint_x_data = &mut vec![0u8; 82];
    let mint_x_owner = spl_token::ID;
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
    let mint_y_data = &mut vec![0u8; 82];
    let mint_y_owner = spl_token::ID;
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

    // Perform the quote calculation
    let quote_result = quote_exact_in(
        test_pair.lb_pair,
        &lb_pair_data,
        amount_in,
        swap_for_y,
        bin_arrays,
        None,
        &clock,
        mint_x_account,
        mint_y_account,
    );

    match quote_result {
        Ok(quote) => {
            println!("Quote exact in successful!");
            println!("Amount out: {}", quote.amount_out);
            println!("Fee: {}", quote.fee);
            
            assert!(quote.amount_out > 0, "Amount out should be greater than 0");
            assert!(quote.fee >= 0, "Fee should be non-negative");
            
            // The amount out should be less than amount in (due to fees)
            assert!(quote.amount_out <= amount_in, "Amount out should be less than or equal to amount in");
        }
        Err(e) => {
            println!("Quote exact in failed: {:?}", e);
            return Err(e);
        }
    }

    println!("On-chain swap exact in test completed successfully!");
    Ok(())
}

/// Helper function to create mock LB pair data
fn create_mock_lb_pair(
    token_x_mint: Pubkey,
    token_y_mint: Pubkey,
    reserve_x: Pubkey,
    reserve_y: Pubkey,
) -> commons::dlmm::types::LbPair {
    use commons::dlmm::types::*;
    use commons::extensions::lb_pair::*;
    
    // Create a mock LbPair with reasonable test data
    let mut lb_pair = LbPair {
        parameters: StaticParameters {
            base_factor: 5000,        // 0.5% base fee
            filter_period: 30,        // 30 seconds
            decay_period: 600,        // 10 minutes
            reduction_factor: 5000,   // 50% reduction
            variable_fee_control: 40000, // 4% max variable fee
            protocol_share: 1000,     // 10% protocol share
            max_volatility_accumulator: 350000, // 35% max volatility
        },
        v_parameters: VariableParameters {
            volatility_accumulator: 0,
            volatility_reference: 0,
            id_reference: 8388608, // ID 2^23 (center bin)
            time_of_last_update: 1700000000,
        },
        bin_step: 25, // 0.25% bin step
        pair_type: PairType::Base as u8,
        active_id: 8388608, // Center bin
        bin_step_seed: [0; 2],
        token_x_mint,
        token_y_mint,
        reserve_x,
        reserve_y,
        protocol_fee: PairProtocolFee {
            amount_x: 0,
            amount_y: 0,
        },
        fees: PairFees {
            protocol_fee_percentage: 1000, // 10%
            base_fee_percentage: 5000,     // 0.5%
        },
        reward_infos: [PairRewardInfo::default(); 2],
        oracle: Pubkey::default(),
        bin_array_bitmap: [0; 512],
        last_updated_at: 1700000000,
        whitelisted_wallet: Pubkey::default(),
        pre_activation_swap_address: Pubkey::default(),
        base_key: Pubkey::default(),
        activation_type: ActivationType::Timestamp as u8,
        padding: [0; 7],
        activation_point: 0,
        pre_activation_duration: 0,
        padding1: [0; 64],
    };

    // Set status to enabled
    let mut pair_status = PairStatus::Enabled as u8;
    lb_pair.pair_type = pair_status;

    lb_pair
}

/// Helper function to create mock bin arrays
fn create_mock_bin_arrays() -> HashMap<Pubkey, commons::dlmm::types::BinArray> {
    use commons::dlmm::types::*;
    
    let mut bin_arrays = HashMap::new();
    
    // Create a mock bin array around the active bin
    let bin_array_key = Pubkey::new_unique();
    let mut bins = [Bin::default(); 70]; // MAX_BIN_PER_ARRAY
    
    // Add some liquidity to a few bins around the center
    for i in 30..40 {
        bins[i] = Bin {
            amount_x: 1000000000, // 1000 tokens
            amount_y: 1000000000000, // 1000 tokens (different decimals)
            price: 1000000, // Mock price
            liquidity_supply: 1000000000,
            reward_per_token_stored: [0; 2],
            fee_amount_x_per_token_stored: 0,
            fee_amount_y_per_token_stored: 0,
        };
    }
    
    let bin_array = BinArray {
        index: 0,
        version: 0,
        padding: [0; 7],
        bins,
    };
    
    bin_arrays.insert(bin_array_key, bin_array);
    bin_arrays
}
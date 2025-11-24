
#[cfg(test)]
mod core_test {
    // use super::*;
    use anchor_lang::prelude::*;
    use std::env;
    // use std::sync::Arc;
    use irma::IRMA_ID;
    use irma::pair_config::PairConfig;
    use irma::position_manager::{AllPosition};
    use irma::pricing::init_pricing;
    use irma::pricing::MAX_BACKING_COUNT;
    use irma::pricing::StateMap;
    use irma::meteora_integration::Core;
    use irma::{MarketMakingMode, Init, Maint, InitBumps, MaintBumps};
    use commons::dlmm::accounts::{LbPair, PositionV2};
    use commons::dlmm::types::{UserRewardInfo, FeeInfo};
    use anchor_lang::prelude::borsh::BorshSerialize;

    // Helper function to create mock AccountInfo
    fn create_mock_account_info<'a>(
        key: &'a Pubkey,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            false, // is_signer
            false, // is_writable  
            lamports,
            data,
            owner,
            false, // executable
            0,     // rent_epoch
        )
    }

    // Usage example:
    // let mut position_data = vec![0u8; std::mem::size_of::<PositionV2>()];
    // let mut lamports = 0u64;
    // let position_pubkey = Pubkey::new_unique();
    // let owner = Pubkey::new_unique();

    // let position_account_info = create_mock_account_info(
    //     &position_pubkey,
    //     &mut lamports,
    //     &mut position_data,
    //     &owner,
    // );

    // Then use in remaining_accounts
    // remaining_accounts: &[position_account_info],

    fn allocate_state() -> StateMap {
        StateMap::new()
    }

    fn create_position(lb_pair: &Pubkey, irma_admin_key: &Pubkey) -> PositionV2 {
        PositionV2 {
            lb_pair: *lb_pair,
            owner: *irma_admin_key,
            liquidity_shares: [10u128; 70],
            reward_infos: [UserRewardInfo::default(); 70],
            fee_infos: [FeeInfo::default(); 70],
            lower_bin_id: -50i32,
            upper_bin_id: 50i32,
            last_updated_at: 0i64,
            total_claimed_fee_x_amount: 0u64,
            total_claimed_fee_y_amount: 0u64,
            total_claimed_rewards: [0u64; 2],
            operator: Pubkey::default(),
            lock_release_point: 0u64,
            _padding_0: 0u8,
            fee_owner: Pubkey::default(),
            _reserved: [0u8; 87],
        }
    }

    fn prep_accounts<'info>(
            owner: &'info Pubkey,
            state_account: Pubkey,
            lb_pair: &'info Pubkey
        ) -> (AccountInfo<'info>, AccountInfo<'info>, AccountInfo<'info>, AccountInfo<'info>) {
        // Create a buffer for StateMap and wrap it in AccountInfo
        let lamports: &mut u64 = Box::leak(Box::new(100000u64));
        let mut state: StateMap = allocate_state();
        let _ = state.init_reserves(); // Add initial stablecoins to the state

        // Prepare the account data with the correct discriminator
        let mut state_data_vec: Vec<u8> = Vec::with_capacity(120*MAX_BACKING_COUNT);
        state.try_serialize(&mut state_data_vec).unwrap();

        let state_data: &'info mut Vec<u8> = Box::leak(Box::new(state_data_vec));
        let state_key: &'info mut Pubkey = Box::leak(Box::new(state_account));
        // msg!("StateMap pre-test account data: {:?}", state_data);
        let state_account_info: AccountInfo<'info> = AccountInfo::new(
            state_key,
            false, // is_signer
            true,  // is_writable
            lamports,
            state_data,
            owner,
            false,
            0,
        );

        let position: PositionV2 = create_position(lb_pair, owner);
        let lamports: &mut u64 = Box::leak(Box::new(100000u64));
        
        // Serialize using bytemuck (for Pod types)
        let position_data_vec = bytemuck::bytes_of(&position).to_vec();
        
        // Add discriminator (8 bytes) at the beginning if needed for account format
        let mut full_data = vec![0u8; 8]; // discriminator
        full_data.extend_from_slice(&position_data_vec);
        
        let position_data: &'info mut Vec<u8> = Box::leak(Box::new(full_data));
        let position_key: &'info mut Pubkey = Box::leak(Box::new(Pubkey::new_unique()));
        let position_account_info: AccountInfo<'info> = AccountInfo::new(
            position_key,
            false, // is_signer
            true,  // is_writable
            lamports,
            position_data,
            owner,
            false,
            0,
        );

        // msg!("StateMap account created: {:?}", state_account_info.key);
        // msg!("StateMap owner: {:?}", owner);
        // Use a mock Signer for testing purposes
        let signer_pubkey: &'info mut Pubkey 
            = Box::leak(Box::new(Pubkey::from_str_const("68bjdGBTr4yRxLW56s7LvpQehMn9jBvaJvV134NQjpmP")));
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
        // Create AccountInfo for system_program
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
        (state_account_info, signer_account_info, sys_account_info, position_account_info)
    }

    fn initialize_anchor<'info>(
        program_id: &'info Pubkey, 
        lb_pair: &'info Pubkey
    ) -> (Account<'info, StateMap>,
            Signer<'info>, 
            Program<'info, anchor_lang::system_program::System>,
            AccountInfo<'info>) {
        //                 state_account_info: &'info AccountInfo<'info>) {
        //                 sys_account_info: &AccountInfo<'info>) {
        // let program_id: &'info Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let state_account: Pubkey = Pubkey::find_program_address(&[b"state".as_ref()], program_id).0;
        let (state_account_info, irma_admin_account_info, sys_account_info, position_account_info) 
                 = prep_accounts(program_id, state_account, lb_pair);
        // Bind to variables to extend their lifetime
        let state_account_static: &'info AccountInfo<'info> = Box::leak(Box::new(state_account_info));
        let irma_admin_account_static: &'info AccountInfo<'info> = Box::leak(Box::new(irma_admin_account_info));
        let sys_account_static: &'info AccountInfo<'info> = Box::leak(Box::new(sys_account_info));
        let mut accounts: Init<'_> = Init {
            state: Account::try_from(state_account_static).unwrap(),
            irma_admin: Signer::try_from(irma_admin_account_static).unwrap(),
            system_program: Program::try_from(sys_account_static).unwrap(),
        };
        let ctx: Context<Init> = Context::new(
            program_id,
            &mut accounts,
            &[],
            InitBumps::default(), // Use default bumps if not needed
        );
        let result: std::result::Result<(), Error> = init_pricing(ctx);
        assert!(result.is_ok());
        // msg!("StateMap account: {:?}", accounts.state);
        return (accounts.state, accounts.irma_admin, accounts.system_program, position_account_info);
    }

    #[test]
    fn test_withdraw() {
        let program_id: &Pubkey = &IRMA_ID;

        let lb_pair = Pubkey::from_str_const("FoSDw2L5DmTuQTFe55gWPDXf88euaxAEKFre74CnvQbX");
        let (state_account, irma_admin_account, sys_account, position_account_info) 
                = initialize_anchor(program_id, &lb_pair);

        let config = vec![PairConfig {
            pair_address: lb_pair.to_string(),
            x_amount: 17000000,
            y_amount: 2000000,
            mode: MarketMakingMode::ModeBoth,
        }];

        let core = &mut Core::new(
            Context {
                program_id: &irma::IRMA_ID,
                accounts: &mut irma::Init {
                    state: state_account.clone(),
                    irma_admin: irma_admin_account.clone(),
                    system_program: sys_account.clone(),
                },
                remaining_accounts: &[position_account_info],
                bumps: InitBumps {
                    ..Default::default()
                },
            },
            irma_admin_account.key(),
            // wallet: Some(Arc::new(payer)),
            config.clone(),
            AllPosition::new(&config).unwrap(),
        );

        let mut accounts: Maint<'_> = Maint {
            state: state_account.clone(),
            irma_admin: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let ctx: Context<Maint> = Context::new(
            program_id,
            &mut accounts,
            &[],
            MaintBumps::default(), // Use default bumps if not needed
        );

        core.refresh_state(&ctx).unwrap();

        let state = core.get_position_state(lb_pair);

        // withdraw
        core.withdraw(&ctx, &state).unwrap();
    }

    #[test]
    fn test_swap() {
        let program_id: &Pubkey = &IRMA_ID;

        let lb_pair = Pubkey::from_str_const("FoSDw2L5DmTuQTFe55gWPDXf88euaxAEKFre74CnvQbX");
        let (state_account, irma_admin_account, sys_account, position_account_info) 
                = initialize_anchor(program_id, &lb_pair);

        let config = vec![PairConfig {
            pair_address: lb_pair.to_string(),
            x_amount: 17000000,
            y_amount: 2000000,
            mode: MarketMakingMode::ModeBoth,
        }];

        let core = &mut Core::new(
            Context {
                program_id: &irma::IRMA_ID,
                accounts: &mut irma::Init {
                    state: state_account.clone(),
                    irma_admin: irma_admin_account.clone(),
                    system_program: sys_account.clone(),
                },
                remaining_accounts: &[position_account_info],
                bumps: InitBumps {
                    ..Default::default()
                },
            },
            irma_admin_account.key(), // owner
            // wallet: Some(Arc::new(payer)),
            config.clone(),
            AllPosition::new(&config).unwrap(),
        );

        let mut accounts: Maint<'_> = Maint {
            state: state_account.clone(),
            irma_admin: irma_admin_account.clone(),
            system_program: sys_account.clone(),
        };
        let ctx: Context<Maint> = Context::new(
            program_id,
            &mut accounts,
            &[],
            MaintBumps::default(), // Use default bumps if not needed
        );

        core.refresh_state(&ctx).unwrap();

        let state = core.get_position_state(lb_pair);

        core.swap(&ctx, &state, 1000000, true).unwrap();
    }
}

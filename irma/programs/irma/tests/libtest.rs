#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anchor_lang::prelude::*;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::prelude::Clock;
    use anchor_lang::prelude::Sysvar;
    use anchor_lang::prelude::Signer;
    // use anchor_lang::prelude::Account;
    use anchor_lang::prelude::Program;
    use anchor_lang::context::Context;
    use anchor_lang::solana_program::sysvar::clock::ID as CLOCK_ID;
    use anchor_lang::system_program;
    use anchor_lang::Accounts;

    use irma::irma as money;
    use irma::pricing::{StateMap, StableState};
    use irma::IRMA_ID;
    use irma::pricing::MAX_BACKING_COUNT;
    use irma::{Init, Common, Maint};
    use irma::pricing::{init_pricing, set_mint_price, mint_irma, redeem_irma, list_reserves};
    use irma::CrankAccounts;
    use irma::State;

    fn prep_accounts(owner: &'static Pubkey, state_account: Pubkey) -> (AccountInfo<'static>, AccountInfo<'static>, AccountInfo<'static>) {
        // Create a buffer for StateMap and wrap it in AccountInfo
        let lamports: &mut u64 = Box::leak(Box::new(100000u64));
        let state: State = State {
            pubkey: Pubkey::new_unique(),
            mint_price: 1.0,
            last_updated: 1640995200, // Use a fixed timestamp for testing
            lamports: 0,
            stablecoin: 0,
            padding1: [0; 7],
            bump: 0,
            padding2: [0; 7],
        };

        // Prepare the account data with the correct discriminator
        let mut state_data_vec: Vec<u8> = Vec::with_capacity(size_of::<State>());
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

    fn initialize_anchor(program_id: &'static Pubkey) -> (Account<'static, State>, Signer<'static>, Program<'static, anchor_lang::system_program::System>) {
        //                 state_account_info: &'static AccountInfo<'static>) {
        //                 sys_account_info: &AccountInfo<'static>) {
        // let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::new_from_array(irma::ID.to_bytes())));
        let state_account: Pubkey = Pubkey::find_program_address(&[b"crank_state".as_ref()], program_id).0;
        let (state_account_info, irma_admin_account_info, sys_account_info) 
                 = prep_accounts(program_id, state_account);
        // Bind to variables to extend their lifetime
        let state_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(state_account_info));
        let irma_admin_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(irma_admin_account_info));
        let sys_account_static: &'static AccountInfo<'static> = Box::leak(Box::new(sys_account_info));
        let mut accounts: CrankAccounts<'_> = CrankAccounts {
            crank_state: Account::try_from(state_account_static).unwrap(),
            irma_admin: Signer::try_from(irma_admin_account_static).unwrap(),
            system_program: Program::try_from(sys_account_static).unwrap(),
        };
        let ctx: Context<CrankAccounts<'_>> = Context::new(
            program_id,
            &mut accounts,
            &[],
            BTreeMap::<String, u8>::default(), // Use default bumps if not needed
        );
        // msg!("StateMap account: {:?}", accounts.state);
        return (accounts.crank_state, accounts.irma_admin, accounts.system_program);
    }

    #[test]
    fn test_crank() -> Result<()> {
        let program_id: &'static Pubkey = &IRMA_ID;
        let (crank_state_account, irma_admin_account, sys_account) 
                = initialize_anchor(program_id);
        let mut accounts: CrankAccounts<'_> = CrankAccounts {
            crank_state: crank_state_account,
            irma_admin: irma_admin_account,
            system_program: sys_account,
        };
        let ctx: Context<CrankAccounts<'_>> = Context::new(
            program_id,
            &mut accounts,
            &[],
            BTreeMap::<String, u8>::default(), // Use default bumps if not needed
        );
        let crank_result: std::result::Result<(), Error> = money::crank(ctx);
        assert!(crank_result.is_ok());
        msg!("Crank executed successfully");
        Ok(())
    }
}

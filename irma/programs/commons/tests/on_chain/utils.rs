use anchor_lang::prelude::*;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::Client;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use anchor_spl::token::spl_token;
use std::rc::Rc;

/// Process instructions and assert success for on-chain tests
pub async fn process_and_assert_ok_on_chain(
    instructions: &[Instruction],
    payer: &Rc<Keypair>,
    signers: &[&Keypair],
    client: &Client,
) -> Result<()> {
    let recent_blockhash = client.rpc().get_latest_blockhash().await
        .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

    let mut all_signers = vec![payer.as_ref()];
    all_signers.extend_from_slice(signers);

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    let result = client.rpc().send_and_confirm_transaction(&tx).await
        .map_err(|e| Error::msg(format!("Transaction failed: {}", e)))?;

    println!("Transaction successful: {}", result);
    Ok(())
}

/// Get or create Associated Token Account on-chain
pub async fn get_or_create_ata_on_chain(
    payer: &Rc<Keypair>,
    token_mint: &Pubkey,
    authority: &Pubkey,
    client: &Client,
) -> Result<Pubkey> {
    // Determine the token program (SPL Token vs Token 2022)
    let mint_account = client.rpc().get_account(token_mint).await
        .map_err(|e| Error::msg(format!("Failed to get mint account: {}", e)))?;
    
    let token_program_id = mint_account.owner;
    
    let ata_address = get_associated_token_address_with_program_id(
        authority, 
        token_mint, 
        &token_program_id
    );
    
    // Check if ATA already exists
    let ata_account = client.rpc().get_account(&ata_address).await;
    
    if ata_account.is_err() || ata_account.unwrap().is_none() {
        create_associated_token_account_on_chain(
            payer,
            token_mint,
            authority,
            &token_program_id,
            client,
        ).await?;
    }
    
    Ok(ata_address)
}

/// Create Associated Token Account on-chain
pub async fn create_associated_token_account_on_chain(
    payer: &Rc<Keypair>,
    token_mint: &Pubkey,
    authority: &Pubkey,
    program_id: &Pubkey,
    client: &Client,
) -> Result<()> {
    println!("Creating ATA for mint: {}, authority: {}, program: {}", 
             token_mint, authority, program_id);
    
    let ins = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        authority,
        token_mint,
        program_id,
    );

    process_and_assert_ok_on_chain(&[ins], payer, &[], client).await
}

/// Wrap SOL into wSOL for testing
pub async fn wrap_sol_on_chain(
    payer: &Rc<Keypair>,
    wallet: &Pubkey,
    amount: u64,
    client: &Client,
) -> Result<()> {
    let wsol_ata = spl_associated_token_account::get_associated_token_address(
        wallet,
        &spl_token::native_mint::id(),
    );

    let create_wsol_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &spl_token::native_mint::id(),
            &spl_token::id(),
        );

    let transfer_sol_ix =
        solana_program::system_instruction::transfer(&payer.pubkey(), &wsol_ata, amount);

    let sync_native_ix = spl_token::instruction::sync_native(&spl_token::id(), &wsol_ata)
        .map_err(|e| Error::msg(format!("Failed to create sync native instruction: {}", e)))?;

    process_and_assert_ok_on_chain(
        &[create_wsol_ata_ix, transfer_sol_ix, sync_native_ix],
        payer,
        &[],
        client,
    ).await
}

/// Get clock data from on-chain
pub async fn get_clock_on_chain(client: &Client) -> Result<solana_program::clock::Clock> {
    let clock_account = client.rpc().get_account(&solana_program::sysvar::clock::id()).await
        .map_err(|e| Error::msg(format!("Failed to get clock account: {}", e)))?
        .ok_or_else(|| Error::msg("Clock account not found"))?;

    let clock_state = bincode::deserialize::<solana_program::clock::Clock>(clock_account.data.as_ref())
        .map_err(|e| Error::msg(format!("Failed to deserialize clock: {}", e)))?;

    Ok(clock_state)
}

/// Helper to create a transaction and get signature for tracking
pub async fn create_and_send_transaction(
    instructions: &[Instruction],
    payer: &Rc<Keypair>,
    signers: &[&Keypair],
    client: &Client,
) -> Result<String> {
    let recent_blockhash = client.rpc().get_latest_blockhash().await
        .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

    let mut all_signers = vec![payer.as_ref()];
    all_signers.extend_from_slice(signers);

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    let signature = client.rpc().send_and_confirm_transaction(&tx).await
        .map_err(|e| Error::msg(format!("Transaction failed: {}", e)))?;

    Ok(signature.to_string())
}

/// Get token account balance
pub async fn get_token_balance(
    client: &Client,
    token_account: &Pubkey,
) -> Result<u64> {
    let account_info = client.rpc().get_token_account_balance(token_account).await
        .map_err(|e| Error::msg(format!("Failed to get token balance: {}", e)))?;
    
    let balance = account_info.amount.parse::<u64>()
        .map_err(|e| Error::msg(format!("Failed to parse balance: {}", e)))?;
    
    Ok(balance)
}

/// Wait for a specific number of slots
pub async fn wait_for_slots(client: &Client, slots: u64) -> Result<()> {
    let start_slot = client.rpc().get_slot().await
        .map_err(|e| Error::msg(format!("Failed to get current slot: {}", e)))?;
    
    let target_slot = start_slot + slots;
    
    loop {
        let current_slot = client.rpc().get_slot().await
            .map_err(|e| Error::msg(format!("Failed to get current slot: {}", e)))?;
        
        if current_slot >= target_slot {
            break;
        }
        
        // Wait a bit before checking again
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_on_chain_utilities() -> Result<()> {
        // Basic test to ensure utilities compile and can be called
        println!("On-chain utilities test placeholder");
        Ok(())
    }
}
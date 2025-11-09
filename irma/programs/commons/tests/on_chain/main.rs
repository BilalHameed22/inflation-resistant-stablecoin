mod test_swap;
mod test_swap_token2022;
mod utils;

pub use test_swap::*;
pub use test_swap_token2022::*;
pub use utils::*;

use anchor_lang::prelude::*;
use anchor_spl::token::*;
use anchor_spl::token_2022::*;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_instruction;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::{Client, Cluster};
use commons::dlmm::accounts::*;
use commons::dlmm::types::*;
use commons::*;
use std::rc::Rc;

/// On-chain test configuration
pub struct OnChainTestConfig {
    pub client: Client,
    pub payer: Rc<Keypair>,
    pub program_id: Pubkey,
}

impl OnChainTestConfig {
    /// Create a new on-chain test configuration
    /// This connects to a local validator or devnet
    pub fn new() -> Self {
        // Use local validator by default, can be changed to devnet/testnet
        let cluster = Cluster::Localnet;
        
        // Create or load payer keypair
        let payer = Rc::new(Keypair::new());
        
        // Create client with commitment
        let client = Client::new_with_options(
            cluster,
            payer.clone(),
            CommitmentConfig::confirmed(),
        );

        Self {
            client,
            payer,
            program_id: commons::dlmm::ID, // Use the DLMM program ID
        }
    }

    /// Airdrop SOL to the payer for testing
    pub async fn airdrop_sol(&self, amount: u64) -> Result<()> {
        let signature = self
            .client
            .request_airdrop(&self.payer.pubkey(), amount)
            .await
            .map_err(|e| Error::msg(format!("Airdrop failed: {}", e)))?;

        // Wait for confirmation
        self.client
            .rpc()
            .confirm_transaction(&signature)
            .await
            .map_err(|e| Error::msg(format!("Airdrop confirmation failed: {}", e)))?;

        Ok(())
    }

    /// Create a new mint account on-chain
    pub async fn create_mint(
        &self,
        mint_authority: &Pubkey,
        freeze_authority: Option<&Pubkey>,
        decimals: u8,
    ) -> Result<Keypair> {
        let mint_keypair = Keypair::new();
        let rent = self.client.rpc().get_minimum_balance_for_rent_exemption(82).await
            .map_err(|e| Error::msg(format!("Failed to get rent: {}", e)))?;

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &mint_keypair.pubkey(),
            rent,
            82, // Mint account size
            &spl_token::ID,
        );

        // Initialize mint instruction
        let initialize_mint_ix = spl_token::instruction::initialize_mint(
            &spl_token::ID,
            &mint_keypair.pubkey(),
            mint_authority,
            freeze_authority,
            decimals,
        ).map_err(|e| Error::msg(format!("Failed to create initialize mint instruction: {}", e)))?;

        // Create and send transaction
        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_mint_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, &mint_keypair],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to create mint: {}", e)))?;

        Ok(mint_keypair)
    }

    /// Create a token account for a mint
    pub async fn create_token_account(
        &self,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Keypair> {
        let token_account_keypair = Keypair::new();
        let rent = self.client.rpc().get_minimum_balance_for_rent_exemption(165).await
            .map_err(|e| Error::msg(format!("Failed to get rent: {}", e)))?;

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &token_account_keypair.pubkey(),
            rent,
            165, // Token account size
            &spl_token::ID,
        );

        // Initialize token account instruction
        let initialize_account_ix = spl_token::instruction::initialize_account(
            &spl_token::ID,
            &token_account_keypair.pubkey(),
            mint,
            owner,
        ).map_err(|e| Error::msg(format!("Failed to create initialize account instruction: {}", e)))?;

        // Create and send transaction
        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_account_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, &token_account_keypair],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to create token account: {}", e)))?;

        Ok(token_account_keypair)
    }

    /// Create a Token 2022 mint account on-chain
    pub async fn create_token_2022_mint(
        &self,
        mint_authority: &Pubkey,
        freeze_authority: Option<&Pubkey>,
        decimals: u8,
    ) -> Result<Keypair> {
        let mint_keypair = Keypair::new();
        let rent = self.client.rpc().get_minimum_balance_for_rent_exemption(165).await
            .map_err(|e| Error::msg(format!("Failed to get rent: {}", e)))?;

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &mint_keypair.pubkey(),
            rent,
            165, // Token 2022 mint account size
            &spl_token_2022::ID,
        );

        // Initialize mint instruction for Token 2022
        let initialize_mint_ix = spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            mint_authority,
            freeze_authority,
            decimals,
        ).map_err(|e| Error::msg(format!("Failed to create initialize mint instruction: {}", e)))?;

        // Create and send transaction
        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_mint_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, &mint_keypair],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to create Token 2022 mint: {}", e)))?;

        Ok(mint_keypair)
    }

    /// Create a Token 2022 mint with transfer fees
    pub async fn create_token_2022_mint_with_transfer_fee(
        &self,
        mint_authority: &Pubkey,
        freeze_authority: Option<&Pubkey>,
        decimals: u8,
        transfer_fee_basis_points: u16,
        max_fee: u64,
    ) -> Result<Keypair> {
        let mint_keypair = Keypair::new();
        let rent = self.client.rpc().get_minimum_balance_for_rent_exemption(200).await
            .map_err(|e| Error::msg(format!("Failed to get rent: {}", e)))?;

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &mint_keypair.pubkey(),
            rent,
            200, // Larger size for extensions
            &spl_token_2022::ID,
        );

        // Initialize mint instruction for Token 2022
        let initialize_mint_ix = spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            mint_authority,
            freeze_authority,
            decimals,
        ).map_err(|e| Error::msg(format!("Failed to create initialize mint instruction: {}", e)))?;

        // Note: In a real implementation, you would add transfer fee extension initialization here
        // For now, we'll just create a basic Token 2022 mint

        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_mint_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, &mint_keypair],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to create Token 2022 mint with transfer fee: {}", e)))?;

        Ok(mint_keypair)
    }

    /// Create a Token 2022 account
    pub async fn create_token_2022_account(
        &self,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Keypair> {
        let token_account_keypair = Keypair::new();
        let rent = self.client.rpc().get_minimum_balance_for_rent_exemption(200).await
            .map_err(|e| Error::msg(format!("Failed to get rent: {}", e)))?;

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &token_account_keypair.pubkey(),
            rent,
            200, // Token 2022 account size with extensions
            &spl_token_2022::ID,
        );

        // Initialize token account instruction for Token 2022
        let initialize_account_ix = spl_token_2022::instruction::initialize_account(
            &spl_token_2022::ID,
            &token_account_keypair.pubkey(),
            mint,
            owner,
        ).map_err(|e| Error::msg(format!("Failed to create initialize account instruction: {}", e)))?;

        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_account_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, &token_account_keypair],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to create Token 2022 account: {}", e)))?;

        Ok(token_account_keypair)
    }

    /// Mint tokens to an account
    pub async fn mint_tokens(
        &self,
        mint: &Pubkey,
        token_account: &Pubkey,
        mint_authority: &Keypair,
        amount: u64,
    ) -> Result<()> {
        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            mint,
            token_account,
            &mint_authority.pubkey(),
            &[],
            amount,
        ).map_err(|e| Error::msg(format!("Failed to create mint_to instruction: {}", e)))?;

        let recent_blockhash = self.client.rpc().get_latest_blockhash().await
            .map_err(|e| Error::msg(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer, mint_authority],
            recent_blockhash,
        );

        self.client.rpc().send_and_confirm_transaction(&transaction).await
            .map_err(|e| Error::msg(format!("Failed to mint tokens: {}", e)))?;

        Ok(())
    }
}

/// Test pair setup for on-chain testing
pub struct OnChainTestPair {
    pub config: OnChainTestConfig,
    pub token_x_mint: Keypair,
    pub token_y_mint: Keypair,
    pub user_token_x: Keypair,
    pub user_token_y: Keypair,
    pub lb_pair: Pubkey,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
}

impl OnChainTestPair {
    /// Setup a new test pair on-chain
    pub async fn new() -> Result<Self> {
        let config = OnChainTestConfig::new();
        
        // Airdrop SOL for testing
        config.airdrop_sol(10_000_000_000).await?; // 10 SOL

        // Create mint authorities
        let mint_authority = Keypair::new();
        
        // Create token mints
        let token_x_mint = config.create_mint(&mint_authority.pubkey(), None, 6).await?;
        let token_y_mint = config.create_mint(&mint_authority.pubkey(), None, 9).await?;

        // Create user token accounts
        let user_token_x = config.create_token_account(&token_x_mint.pubkey(), &config.payer.pubkey()).await?;
        let user_token_y = config.create_token_account(&token_y_mint.pubkey(), &config.payer.pubkey()).await?;

        // Mint some tokens for testing
        config.mint_tokens(&token_x_mint.pubkey(), &user_token_x.pubkey(), &mint_authority, 1_000_000_000).await?;
        config.mint_tokens(&token_y_mint.pubkey(), &user_token_y.pubkey(), &mint_authority, 1_000_000_000_000).await?;

        // For now, these will be derived addresses - in a real setup you'd create the LB pair
        let lb_pair = Pubkey::find_program_address(
            &[
                b"lb_pair",
                token_x_mint.pubkey().as_ref(),
                token_y_mint.pubkey().as_ref(),
            ],
            &config.program_id,
        ).0;

        let reserve_x = Pubkey::find_program_address(
            &[b"reserve_x", lb_pair.as_ref()],
            &config.program_id,
        ).0;

        let reserve_y = Pubkey::find_program_address(
            &[b"reserve_y", lb_pair.as_ref()],
            &config.program_id,
        ).0;

        Ok(Self {
            config,
            token_x_mint,
            token_y_mint,
            user_token_x,
            user_token_y,
            lb_pair,
            reserve_x,
            reserve_y,
        })
    }
}
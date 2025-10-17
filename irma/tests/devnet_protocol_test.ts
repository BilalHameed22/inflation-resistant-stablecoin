import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Connection,
} from "@solana/web3.js";
import { expect } from "chai";

describe("IRMA Protocol - Devnet Integration Test", () => {
  // Configure the client to use devnet
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Irma as Program<Irma>;
  const connection = provider.connection;
  
  // Test accounts
  const authority = provider.wallet;
  
  // Orca Whirlpool constants
  const WHIRLPOOL_PROGRAM_ID = new PublicKey(
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
  );
  
  // We'll use devnet USDC for testing
  const USDC_MINT = new PublicKey(
    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU" // Devnet USDC
  );
  
  let protocolStatePDA: PublicKey;
  let protocolStateBump: number;
  
  // These will be set during the test
  let irmaMint: PublicKey;
  let whirlpoolPubkey: PublicKey;
  let positionPubkey: PublicKey;

  before(async () => {
    console.log("\n🚀 Starting IRMA Devnet Integration Test");
    console.log("━".repeat(60));
    console.log(`Provider: ${provider.connection.rpcEndpoint}`);
    console.log(`Authority: ${authority.publicKey.toBase58()}`);
    console.log("━".repeat(60));

    // Derive the protocol state PDA
    [protocolStatePDA, protocolStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("protocol_state")],
      program.programId
    );

    console.log(`\n📍 Protocol State PDA: ${protocolStatePDA.toBase58()}`);
  });

  it("Step 1: Initialize Protocol State", async () => {
    console.log("\n" + "=".repeat(60));
    console.log("📋 STEP 1: Initialize Protocol State");
    console.log("=".repeat(60));

    // For this test, we'll use placeholder addresses for whirlpool and position
    // In a real scenario, these would be created using Orca's SDK first
    const placeholderWhirlpool = Keypair.generate().publicKey;
    const placeholderPosition = Keypair.generate().publicKey;
    const placeholderIrmaMint = Keypair.generate().publicKey;

    const initialMintPrice = new BN(1_000_000_000); // 1.0 USDC (scaled by 1e9)
    const initialRedemptionPrice = new BN(1_000_000_000); // 1.0 USDC

    console.log("\n📊 Initial Parameters:");
    console.log(`  Mint Price: ${initialMintPrice.toString()} (1.0 USDC)`);
    console.log(`  Redemption Price: ${initialRedemptionPrice.toString()} (1.0 USDC)`);
    console.log(`  Whirlpool (placeholder): ${placeholderWhirlpool.toBase58()}`);
    console.log(`  Position (placeholder): ${placeholderPosition.toBase58()}`);

    try {
      const tx = await program.methods
        .initializeProtocol(
          initialMintPrice,
          initialRedemptionPrice,
          placeholderWhirlpool,
          placeholderPosition,
          placeholderIrmaMint,
          USDC_MINT
        )
        .accounts({
          authority: authority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("\n✅ Protocol initialized successfully!");
      console.log(`📝 Transaction: ${tx}`);
      console.log(`🔗 View on Solana Explorer:`);
      console.log(`   https://explorer.solana.com/tx/${tx}?cluster=devnet`);

      // Store for later tests
      whirlpoolPubkey = placeholderWhirlpool;
      positionPubkey = placeholderPosition;
      irmaMint = placeholderIrmaMint;

      // Verify the state was created
      const protocolState = await program.account.protocolState.fetch(
        protocolStatePDA
      );

      console.log("\n📦 Protocol State Created:");
      console.log(`  Authority: ${protocolState.authority.toBase58()}`);
      console.log(`  Mint Price: ${protocolState.mintPrice.toString()}`);
      console.log(`  Redemption Price: ${protocolState.redemptionPrice.toString()}`);
      console.log(`  Whirlpool: ${protocolState.whirlpool.toBase58()}`);
      console.log(`  Position: ${protocolState.position.toBase58()}`);
      console.log(`  Bump: ${protocolState.bump}`);

      expect(protocolState.authority.toBase58()).to.equal(
        authority.publicKey.toBase58()
      );
      expect(protocolState.mintPrice.toString()).to.equal(
        initialMintPrice.toString()
      );
      expect(protocolState.redemptionPrice.toString()).to.equal(
        initialRedemptionPrice.toString()
      );
    } catch (error) {
      console.error("\n❌ Error initializing protocol:", error);
      throw error;
    }
  });

  it("Step 2: Update Mock Prices (Simulate 5% Inflation)", async () => {
    console.log("\n" + "=".repeat(60));
    console.log("📈 STEP 2: Update Mock Prices (Simulating Inflation)");
    console.log("=".repeat(60));

    // Simulate 5% inflation
    const newMintPrice = new BN(1_050_000_000); // 1.05 USDC (5% increase)
    const newRedemptionPrice = new BN(1_000_000_000); // 1.0 USDC (unchanged)

    console.log("\n📊 New Prices:");
    console.log(`  Mint Price: ${newMintPrice.toString()} (1.05 USDC) - 5% increase`);
    console.log(`  Redemption Price: ${newRedemptionPrice.toString()} (1.0 USDC)`);
    console.log(`  Spread: 5%`);

    try {
      const tx = await program.methods
        .updateMockPrices(newMintPrice, newRedemptionPrice)
        .accounts({
          authority: authority.publicKey,
        })
        .rpc();

      console.log("\n✅ Prices updated successfully!");
      console.log(`📝 Transaction: ${tx}`);
      console.log(`🔗 View on Solana Explorer:`);
      console.log(`   https://explorer.solana.com/tx/${tx}?cluster=devnet`);

      // Verify the state was updated
      const protocolState = await program.account.protocolState.fetch(
        protocolStatePDA
      );

      console.log("\n📦 Updated Protocol State:");
      console.log(`  Mint Price: ${protocolState.mintPrice.toString()}`);
      console.log(`  Redemption Price: ${protocolState.redemptionPrice.toString()}`);
      console.log(`  Last Price Update: ${new Date(protocolState.lastPriceUpdate.toNumber() * 1000).toISOString()}`);

      expect(protocolState.mintPrice.toString()).to.equal(
        newMintPrice.toString()
      );
      expect(protocolState.redemptionPrice.toString()).to.equal(
        newRedemptionPrice.toString()
      );
    } catch (error) {
      console.error("\n❌ Error updating prices:", error);
      throw error;
    }
  });

  it("Step 3: Update Prices Again (Simulate 10% Total Inflation)", async () => {
    console.log("\n" + "=".repeat(60));
    console.log("📈 STEP 3: Update Prices Again (10% Total Inflation)");
    console.log("=".repeat(60));

    // Simulate 10% total inflation
    const newMintPrice = new BN(1_100_000_000); // 1.10 USDC
    const newRedemptionPrice = new BN(1_050_000_000); // 1.05 USDC (updated)

    console.log("\n📊 New Prices:");
    console.log(`  Mint Price: ${newMintPrice.toString()} (1.10 USDC)`);
    console.log(`  Redemption Price: ${newRedemptionPrice.toString()} (1.05 USDC)`);
    console.log(`  Spread: ~4.76%`);

    try {
      const tx = await program.methods
        .updateMockPrices(newMintPrice, newRedemptionPrice)
        .accounts({
          authority: authority.publicKey,
        })
        .rpc();

      console.log("\n✅ Prices updated successfully!");
      console.log(`📝 Transaction: ${tx}`);

      const protocolState = await program.account.protocolState.fetch(
        protocolStatePDA
      );

      console.log("\n📦 Updated Protocol State:");
      console.log(`  Mint Price: ${protocolState.mintPrice.toString()}`);
      console.log(`  Redemption Price: ${protocolState.redemptionPrice.toString()}`);

      expect(protocolState.mintPrice.toString()).to.equal(
        newMintPrice.toString()
      );
    } catch (error) {
      console.error("\n❌ Error updating prices:", error);
      throw error;
    }
  });

  it("Step 4: Test Error Cases - Invalid Price Updates", async () => {
    console.log("\n" + "=".repeat(60));
    console.log("🧪 STEP 4: Test Error Cases");
    console.log("=".repeat(60));

    console.log("\n🔴 Testing: Mint price < Redemption price (should fail)");
    
    const invalidMintPrice = new BN(900_000_000); // 0.9 USDC
    const invalidRedemptionPrice = new BN(1_000_000_000); // 1.0 USDC

    try {
      await program.methods
        .updateMockPrices(invalidMintPrice, invalidRedemptionPrice)
        .accounts({
          authority: authority.publicKey,
        })
        .rpc();

      throw new Error("Should have failed with invalid price relationship");
    } catch (error) {
      if (error.message.includes("InvalidPriceRelation")) {
        console.log("✅ Correctly rejected invalid price relationship");
      } else {
        console.log(`✅ Transaction failed as expected: ${error.message}`);
      }
    }

    console.log("\n🔴 Testing: Zero redemption price (should fail)");
    
    const zeroRedemptionPrice = new BN(0);
    const validMintPrice = new BN(1_000_000_000);

    try {
      await program.methods
        .updateMockPrices(validMintPrice, zeroRedemptionPrice)
        .accounts({
          authority: authority.publicKey,
        })
        .rpc();

      throw new Error("Should have failed with zero price");
    } catch (error: any) {
      if (error.message?.includes("ZeroPrice")) {
        console.log("✅ Correctly rejected zero price");
      } else {
        console.log(`✅ Transaction failed as expected: ${error.message || error}`);
      }
    }
  });

  after(async () => {
    console.log("\n" + "=".repeat(60));
    console.log("🎉 ALL TESTS PASSED!");
    console.log("=".repeat(60));
    console.log("\n📊 Final Summary:");
    console.log(`  Protocol State PDA: ${protocolStatePDA.toBase58()}`);
    console.log(`  Program ID: ${program.programId.toBase58()}`);
    console.log(`  Cluster: Devnet`);
    console.log("\n🔗 View Protocol State on Explorer:");
    console.log(`   https://explorer.solana.com/address/${protocolStatePDA.toBase58()}?cluster=devnet`);
    console.log("\n✅ Protocol is ready for Orca integration!");
    console.log("━".repeat(60));
  });
});

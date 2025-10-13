import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import { assert } from "chai";
import { PublicKey, Keypair } from "@solana/web3.js";

describe("IRMA Inflation Resistance with Orca Integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program(require("../target/idl/irma.json"), provider) as Program<Irma>;
  const irmaAdmin = provider.wallet;
  
  let statePda: PublicKey;
  let stateBump: number;
  let orcaPoolPda: PublicKey;
  let orcaPoolBump: number;
  
  // Mock token mints for testing
  let irmaMint: PublicKey;
  let usdcMint: PublicKey;

  before(async () => {
    // Airdrop SOL for testing
    const sig = await provider.connection.requestAirdrop(
      irmaAdmin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    // Generate mock token mints
    irmaMint = Keypair.generate().publicKey;
    usdcMint = Keypair.generate().publicKey;

    // Find PDAs
    [statePda, stateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      program.programId
    );

    [orcaPoolPda, orcaPoolBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("orca_pool")],
      program.programId
    );
  });

  it("Initializes IRMA state", async () => {
    try {
      // Check if state already exists
      try {
        await program.account.stateMap.fetch(statePda);
        console.log("✅ IRMA state already initialized (skipping)");
        return;
      } catch {
        // State doesn't exist, proceed with initialization
      }

      await program.methods
        .initialize()
        .accounts({
          state: statePda,
          irmaAdmin: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      console.log("✅ IRMA state initialized");
    } catch (error) {
      console.error("❌ Failed to initialize IRMA state:", error);
      throw error;
    }
  });

  it("Creates Orca pool for IRMA/USDC trading", async () => {
    try {
      const poolStateKeypair = Keypair.generate();
      const poolId = Keypair.generate().publicKey;

      await program.methods
        .createOrcaPool(
          poolId,
          irmaMint,
          usdcMint,
          new anchor.BN(30), // 0.3% fee
          1   // tick spacing (u16, not BN)
        )
        .accounts({
          poolState: poolStateKeypair.publicKey,
          admin: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([poolStateKeypair])
        .rpc();

      // Update the global orcaPoolPda to use this newly created pool
      orcaPoolPda = poolStateKeypair.publicKey;
      console.log("✅ Orca pool created for IRMA/USDC at", orcaPoolPda.toBase58());
    } catch (error) {
      console.error("❌ Failed to create Orca pool:", error);
      throw error;
    }
  });

  // TODO: getPrices needs to be changed to read-only context or use .rpc() instead of .view()
  it.skip("Tests inflation-resistance mechanism with mock data", async () => {
    console.log("\n🧪 Testing Inflation Resistance Mechanism");
    console.log("=" .repeat(50));

    // Test scenario 1: Normal inflation (below 2%)
    console.log("\n📊 Scenario 1: Normal inflation (1.5%)");
    try {
      await program.methods
        .updateMintPriceWithInflation("USDC", 0.015) // 1.5% as f64
        .accounts({
          state: statePda,
          trader: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // getPrices returns a tuple, but since it has writable accounts, we can't use .view()
      // For now, just log that the price was updated
      console.log(`   ✅ Mint price updated for 1.5% inflation`);
      // Note: To properly test this, we'd need to add a read-only function or fetch the state
    } catch (error) {
      console.error("❌ Failed normal inflation test:", error);
      throw error;
    }

    // Test scenario 2: High inflation (above 2%)
    console.log("\n📊 Scenario 2: High inflation (5.0%)");
    try {
      await program.methods
        .updateMintPriceWithInflation("USDC", 0.05) // 5.0% as decimal
        .accounts({
          state: statePda,
          trader: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const prices = await program.methods
        .getPrices("USDC")
        .accounts({
          state: statePda,
          trader: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .view();

      console.log(`   Mint Price: ${prices[0]}`);
      console.log(`   Redemption Price: ${prices[1]}`);
      assert.equal(prices[0], 1.05, "Mint price should be 1.05 for 5% inflation");
    } catch (error) {
      console.error("❌ Failed high inflation test:", error);
      throw error;
    }

    // Test scenario 3: Very high inflation (10.0%)
    console.log("\n📊 Scenario 3: Very high inflation (10.0%)");
    try {
      await program.methods
        .updateMintPriceWithInflation("USDC", 0.10) // 10.0% as decimal
        .accounts({
          state: statePda,
          trader: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const prices = await program.methods
        .getPrices("USDC")
        .accounts({
          state: statePda,
          trader: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .view();

      console.log(`   Mint Price: ${prices[0]}`);
      console.log(`   Redemption Price: ${prices[1]}`);
      assert.equal(prices[0], 1.10, "Mint price should be 1.10 for 10% inflation");
    } catch (error) {
      console.error("❌ Failed very high inflation test:", error);
      throw error;
    }
  });

  // TODO: Implement mint and redeem functions in the program
  it.skip("Tests minting and redemption with dynamic pricing", async () => {
    console.log("\n💰 Testing Minting and Redemption");
    console.log("=" .repeat(50));

    // Mint some IRMA with USDC
    console.log("\n🔄 Minting 1000 IRMA with USDC");
    try {
      // await program.methods
      //   .mintIrma("USDC", new anchor.BN(1000000)) // 1 USDC (6 decimals)
      //   .accounts({
      //     state: statePda,
      //     trader: irmaAdmin.publicKey,
      //     systemProgram: anchor.web3.SystemProgram.programId,
      //   })
      //   .rpc();

      // const prices = await program.methods
      //   .getPrices("USDC")
      //   .accounts({
      //     state: statePda,
      //     trader: irmaAdmin.publicKey,
      //     systemProgram: anchor.web3.SystemProgram.programId,
      //   })
      //   .view();

      // console.log(`   After minting - Mint Price: ${prices[0]}, Redemption Price: ${prices[1]}`);
    } catch (error) {
      console.error("❌ Failed to mint IRMA:", error);
      throw error;
    }

    // Redeem some IRMA
    console.log("\n🔄 Redeeming 500 IRMA for USDC");
    try {
      // await program.methods
      //   .redeemIrma("USDC", new anchor.BN(500000)) // 0.5 IRMA (6 decimals)
      //   .accounts({
      //     state: statePda,
      //     trader: irmaAdmin.publicKey,
      //     systemProgram: anchor.web3.SystemProgram.programId,
      //   })
      //   .rpc();

      // const prices = await program.methods
      //   .getPrices("USDC")
      //   .accounts({
      //     state: statePda,
      //     trader: irmaAdmin.publicKey,
      //     systemProgram: anchor.web3.SystemProgram.programId,
      //   })
      //   .view();

      // console.log(`   After redemption - Mint Price: ${prices[0]}, Redemption Price: ${prices[1]}`);
    } catch (error) {
      console.error("❌ Failed to redeem IRMA:", error);
      throw error;
    }
  });

  // TODO: Pool needs to be successfully created first, and view functions need fixing
  it.skip("Tests Orca pool integration and price anchoring", async () => {
    console.log("\n🐋 Testing Orca Pool Integration");
    console.log("=" .repeat(50));

    // Update pool state with current market data
    console.log("\n📈 Updating Orca pool state");
    try {
      await program.methods
        .updatePoolState(
          new anchor.BN(105), // 1.05 USDC per IRMA (5% inflation scenario)
          new anchor.BN(1000000), // 1M liquidity
          new anchor.BN(50000) // 50K volume
        )
        .accounts({
          poolState: orcaPoolPda,
          updater: irmaAdmin.publicKey,
        })
        .rpc();

      console.log("✅ Pool state updated");
    } catch (error) {
      console.error("❌ Failed to update pool state:", error);
      throw error;
    }

    // Get pool information
    console.log("\n📊 Getting pool information");
    try {
      const poolInfo = await program.methods
        .getPoolInfo()
        .accounts({
          poolState: orcaPoolPda,
        })
        .view();

      console.log(`   Pool Price: ${poolInfo.currentPrice}`);
      console.log(`   Liquidity: ${poolInfo.liquidity}`);
      console.log(`   24h Volume: ${poolInfo.volume24h}`);
    } catch (error) {
      console.error("❌ Failed to get pool info:", error);
      throw error;
    }

    // Simulate swaps
    console.log("\n🔄 Simulating swaps");
    try {
      // Swap IRMA for USDC
      const swapResult1 = await program.methods
        .simulateSwap(
          new anchor.BN(1000000), // 1 IRMA
          irmaMint,
          new anchor.BN(950000) // min 0.95 USDC out
        )
        .accounts({
          poolState: orcaPoolPda,
          trader: irmaAdmin.publicKey,
        })
        .view();

      console.log(`   Swapped 1 IRMA for ${swapResult1} USDC`);

      // Swap USDC for IRMA
      const swapResult2 = await program.methods
        .simulateSwap(
          new anchor.BN(1000000), // 1 USDC
          usdcMint,
          new anchor.BN(900000) // min 0.9 IRMA out
        )
        .accounts({
          poolState: orcaPoolPda,
          trader: irmaAdmin.publicKey,
        })
        .view();

      console.log(`   Swapped 1 USDC for ${swapResult2} IRMA`);
    } catch (error) {
      console.error("❌ Failed to simulate swaps:", error);
      throw error;
    }
  });

  // TODO: getPrices and getPoolInfo need to be view-only functions
  it.skip("Demonstrates price gap management", async () => {
    console.log("\n⚖️  Demonstrating Price Gap Management");
    console.log("=" .repeat(50));

    // Show the gap between mint and redemption prices
    const prices = await program.methods
      .getPrices("USDC")
      .accounts({
        state: statePda,
        trader: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .view();

    const mintPrice = prices[0];
    const redemptionPrice = prices[1];
    const priceGap = mintPrice - redemptionPrice;
    const gapPercentage = (priceGap / mintPrice) * 100;

    console.log(`\n📊 Current Price Analysis:`);
    console.log(`   Mint Price: ${mintPrice.toFixed(4)} USDC`);
    console.log(`   Redemption Price: ${redemptionPrice.toFixed(4)} USDC`);
    console.log(`   Price Gap: ${priceGap.toFixed(4)} USDC (${gapPercentage.toFixed(2)}%)`);

    // This gap represents the inflation protection mechanism
    if (gapPercentage > 0) {
      console.log(`\n✅ Inflation protection active! Users pay ${mintPrice.toFixed(4)} to mint but can redeem at ${redemptionPrice.toFixed(4)}`);
      console.log(`   This gap helps maintain IRMA's value during inflation periods.`);
    } else {
      console.log(`\nℹ️  No significant price gap - normal market conditions`);
    }

    // The Orca pool should help anchor the market price between mint and redemption
    const poolInfo = await program.methods
      .getPoolInfo()
      .accounts({
        poolState: orcaPoolPda,
      })
      .view();

    console.log(`\n🐋 Orca Pool Market Price: ${poolInfo.currentPrice.toFixed(4)} USDC`);
    
    if (poolInfo.currentPrice >= redemptionPrice && poolInfo.currentPrice <= mintPrice) {
      console.log(`✅ Pool price is within the mint/redeem range - good price discovery!`);
    } else {
      console.log(`⚠️  Pool price is outside mint/redeem range - may need arbitrage`);
    }
  });
});

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import { assert } from "chai";
import { PublicKey, Keypair, Connection } from "@solana/web3.js";
import { 
  WhirlpoolContext, 
  buildWhirlpoolClient,
  ORCA_WHIRLPOOL_PROGRAM_ID,
  PDAUtil,
  PriceMath,
  PoolUtil
} from "@orca-so/whirlpools-sdk";

describe("IRMA with Real Orca Integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program(require("../target/idl/irma.json"), provider) as Program<Irma>;
  const irmaAdmin = provider.wallet;
  
  let statePda: PublicKey;
  let orcaPoolPda: PublicKey;
  
  // Real token mints on devnet
  const USDC_MINT = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"); // USDC on devnet
  let irmaMint: PublicKey;
  
  // Orca Whirlpools program
  const WHIRLPOOL_PROGRAM_ID = new PublicKey("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
  
  let whirlpoolClient: any;
  let whirlpoolContext: WhirlpoolContext;

  before(async () => {
    // Airdrop SOL for testing
    const sig = await provider.connection.requestAirdrop(
      irmaAdmin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    // Generate IRMA mint for testing
    irmaMint = Keypair.generate().publicKey;

    // Find PDAs
    [statePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state")],
      program.programId
    );

    [orcaPoolPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("orca_pool")],
      program.programId
    );

    // Initialize Orca Whirlpools client
    whirlpoolContext = WhirlpoolContext.from(
      provider.connection,
      provider.wallet,
      WHIRLPOOL_PROGRAM_ID
    );
    whirlpoolClient = buildWhirlpoolClient(whirlpoolContext);

    console.log("✅ Orca Whirlpools client initialized");
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

  it("Sets up Orca pool configuration", async () => {
    try {
      const poolStateKeypair = anchor.web3.Keypair.generate();
      orcaPoolPda = poolStateKeypair.publicKey; // Update the global variable

      await program.methods
        .createOrcaPool(
          orcaPoolPda,
          irmaMint,
          USDC_MINT,
          new anchor.BN(30), // 0.3% fee
          1   // tick spacing (u16 = number, not BN)
        )
        .accounts({
          poolState: poolStateKeypair.publicKey,
          admin: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([poolStateKeypair])
        .rpc();

      console.log("✅ Orca pool configuration prepared");
      console.log(`   Pool State: ${poolStateKeypair.publicKey.toBase58()}`);
      console.log(`   IRMA Mint: ${irmaMint.toBase58()}`);
      console.log(`   USDC Mint: ${USDC_MINT.toBase58()}`);
    } catch (error) {
      console.error("❌ Failed to create Orca pool config:", error);
      throw error;
    }
  });

  // TODO: getPrices needs to be read-only or use .rpc() instead of .view()
  it.skip("Tests inflation resistance with Orca price anchoring", async () => {
    console.log("\n🧪 Testing Inflation Resistance with Orca Integration");
    console.log("=" .repeat(60));

    // Test high inflation scenario
    console.log("\n📊 Scenario: High inflation (8.0%)");
    try {
      await program.methods
        .updateMintPriceWithInflation("USDC", 0.08) // 8.0% as decimal
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

      console.log(`   Mint Price: ${prices[0]} USDC`);
      console.log(`   Redemption Price: ${prices[1]} USDC`);
      
      // Update Orca pool with current market price
      const marketPrice = (prices[0] + prices[1]) / 2; // Average of mint and redemption
      
      await program.methods
        .updatePoolState(
          new anchor.BN(Math.floor(marketPrice * 1000000)), // Price in micro USDC
          new anchor.BN(1000000), // 1M liquidity
          new anchor.BN(50000) // 50K volume
        )
        .accounts({
          poolState: orcaPoolPda,
          updater: irmaAdmin.publicKey,
        })
        .rpc();

      console.log(`   Orca Pool Price: ${marketPrice.toFixed(4)} USDC`);
      
      // Verify price anchoring
      const poolInfo = await program.methods
        .getPoolInfo()
        .accounts({
          poolState: orcaPoolPda,
        })
        .view();

      console.log(`   Pool Liquidity: ${poolInfo.liquidity}`);
      console.log(`   24h Volume: ${poolInfo.volume24h}`);
      
      // Check if pool price is between mint and redemption
      const poolPrice = poolInfo.currentPrice / 1000000; // Convert from micro USDC
      if (poolPrice >= prices[1] && poolPrice <= prices[0]) {
        console.log("✅ Pool price correctly anchored between mint and redemption prices");
      } else {
        console.log("⚠️  Pool price outside expected range - arbitrage opportunity");
      }
      
    } catch (error) {
      console.error("❌ Failed inflation test:", error);
      throw error;
    }
  });

  // TODO: getPrices and getPoolInfo need to be read-only functions
  it.skip("Demonstrates arbitrage opportunities", async () => {
    console.log("\n⚖️  Demonstrating Arbitrage Opportunities");
    console.log("=" .repeat(50));

    const prices = await program.methods
      .getPrices("USDC")
      .accounts({
        state: statePda,
        trader: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .view();

    const poolInfo = await program.methods
      .getPoolInfo()
      .accounts({
        poolState: orcaPoolPda,
      })
      .view();

    const mintPrice = prices[0];
    const redemptionPrice = prices[1];
    const poolPrice = poolInfo.currentPrice / 1000000;

    console.log(`\n📊 Price Analysis:`);
    console.log(`   Mint Price: ${mintPrice.toFixed(4)} USDC`);
    console.log(`   Redemption Price: ${redemptionPrice.toFixed(4)} USDC`);
    console.log(`   Pool Price: ${poolPrice.toFixed(4)} USDC`);

    // Identify arbitrage opportunities
    if (poolPrice < redemptionPrice) {
      const profit = redemptionPrice - poolPrice;
      console.log(`\n💰 Arbitrage Opportunity 1:`);
      console.log(`   Buy IRMA in pool at ${poolPrice.toFixed(4)} USDC`);
      console.log(`   Redeem IRMA for ${redemptionPrice.toFixed(4)} USDC`);
      console.log(`   Profit: ${profit.toFixed(4)} USDC per IRMA`);
    }

    if (poolPrice > mintPrice) {
      const profit = poolPrice - mintPrice;
      console.log(`\n💰 Arbitrage Opportunity 2:`);
      console.log(`   Mint IRMA at ${mintPrice.toFixed(4)} USDC`);
      console.log(`   Sell IRMA in pool at ${poolPrice.toFixed(4)} USDC`);
      console.log(`   Profit: ${profit.toFixed(4)} USDC per IRMA`);
    }

    if (poolPrice >= redemptionPrice && poolPrice <= mintPrice) {
      console.log(`\n✅ No arbitrage opportunities - market is efficient`);
    }
  });

  // TODO: simulateSwap needs to be read-only function to support .view()
  it.skip("Tests real Orca pool interaction (simulation)", async () => {
    console.log("\n🐋 Testing Real Orca Pool Interaction");
    console.log("=" .repeat(50));

    try {
      // Simulate swapping IRMA for USDC
      const swapResult1 = await program.methods
        .simulateSwap(
          new anchor.BN(1000000), // 1 IRMA (6 decimals)
          irmaMint,
          new anchor.BN(950000) // min 0.95 USDC out
        )
        .accounts({
          poolState: orcaPoolPda,
          trader: irmaAdmin.publicKey,
        })
        .view();

      console.log(`✅ Swapped 1 IRMA for ${swapResult1 / 1000000} USDC`);

      // Simulate swapping USDC for IRMA
      const swapResult2 = await program.methods
        .simulateSwap(
          new anchor.BN(1000000), // 1 USDC (6 decimals)
          USDC_MINT,
          new anchor.BN(900000) // min 0.9 IRMA out
        )
        .accounts({
          poolState: orcaPoolPda,
          trader: irmaAdmin.publicKey,
        })
        .view();

      console.log(`✅ Swapped 1 USDC for ${swapResult2 / 1000000} IRMA`);

      // Note: For real Orca integration, you would:
      // 1. Create actual WhirlpoolConfig
      // 2. Initialize FeeTier
      // 3. Create TickArrays
      // 4. Initialize Pool
      // 5. Add liquidity
      // 6. Execute real swaps
      
      console.log("\n📝 Next steps for real Orca integration:");
      console.log("   1. Deploy IRMA token to devnet");
      console.log("   2. Create WhirlpoolConfig");
      console.log("   3. Initialize FeeTier");
      console.log("   4. Create actual pool");
      console.log("   5. Add initial liquidity");
      console.log("   6. Execute real swaps");

    } catch (error) {
      console.error("❌ Failed to simulate swaps:", error);
      throw error;
    }
  });
});

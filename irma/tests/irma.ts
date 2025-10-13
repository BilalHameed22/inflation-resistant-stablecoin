import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import { assert } from "chai";

describe("irma", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  console.log("Using provider:", provider.wallet.publicKey.toBase58());
  const programId = new anchor.web3.PublicKey("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ");
  
  // Load program without strict typing to avoid IDL account issues
  const idl = require("../target/idl/irma.json");
  const program = new Program(idl, provider) as any;
  
  if (!program) throw new Error("Program 'irma' not found in anchor.workspace");

  const irmaAdmin = provider.wallet;
  const stateSeed = Buffer.from("state");
  let statePda: anchor.web3.PublicKey;
  let stateBump: number;

  before(async () => {
    // Find PDA for state
    [statePda, stateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [stateSeed],
      program.programId
    );

    // Check if state already exists
    try {
      await program.account.stateMap.fetch(statePda);
      console.log("✅ IRMA state already initialized at:", statePda.toBase58());
    } catch (error) {
      // State doesn't exist, initialize it
      console.log("Initializing IRMA state...");
      const sig = await provider.connection.requestAirdrop(
        irmaAdmin.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      await program.methods
        .initialize()
        .accounts({
          state: statePda,
          irmaAdmin: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      console.log("✅ IRMA state initialized at:", statePda.toBase58());
    }
  });

  it("Verifies IRMA state is initialized", async () => {
    const stateAccount = await program.account.stateMap.fetch(statePda);
    assert.isDefined(stateAccount);
    console.log("✅ IRMA state verified");
  });

  it("Adds a stablecoin", async () => {
    const symbol = "USDC";
    const mintAddress = anchor.web3.Keypair.generate().publicKey;
    const decimals = 6;

    console.log("Adding stablecoin:", symbol, "Mint Address:", mintAddress.toBase58(), "Decimals:", decimals);

    try {
      await program.methods
        .addReserve(symbol, mintAddress, decimals)
        .accounts({
          state: statePda,
          irmaAdmin: irmaAdmin.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
      
      console.log("Stablecoin added:", symbol);

      // Fetch and check state changes as needed
      const stateAccount = await program.account.stateMap.fetch(statePda);
      assert.isDefined(stateAccount);
      // Add more assertions as needed
    } catch (error) {
      // ✅ Catch SendTransactionError and get full logs
      if (error instanceof anchor.web3.SendTransactionError) {
        const logs = await error.getLogs(provider.connection);
        console.error("Transaction failed with logs:", logs);
        throw new Error(`Transaction failed: ${logs?.join('\n')}`);
      } else {
        console.error("Unexpected error:", error);
        throw error;
      }
    }
  });

  // Crank function has been removed - skipping test
});

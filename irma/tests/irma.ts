import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import { assert } from "chai";

describe("irma", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  console.log("Using provider:", provider.wallet.publicKey.toBase58());
  const programId = new anchor.web3.PublicKey("4rVQnE69m14Qows2iwcgokb59nx7G49VD6fQ9GH9Y6KJ"); // Replace with your actual program ID
  const program = new Program(require("../target/idl/irma.json"), programId, provider) as Program<Irma>;
  // const program = anchor.workspace.irma as Program<Irma>; // see how this is done for anchor 0.27.0
  if (!program) throw new Error("Program 'irma' not found in anchor.workspace");

  // Example keypairs
  const irmaAdmin = provider.wallet; // anchor.web3.Keypair.generate();
  const stateSeed = Buffer.from("state");
  const crankSeed = Buffer.from("crank_state");
  let statePda: anchor.web3.PublicKey;
  let crankPda: anchor.web3.PublicKey;
  let stateBump: number;

  before(async () => {
    // Airdrop SOL to irmaAdmin for testing
    const sig = await provider.connection.requestAirdrop(
      irmaAdmin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    const config = "confirmed" as anchor.web3.Commitment;
    await provider.connection.confirmTransaction(sig, config);
  });

  it("Initializes IRMA state, then adds a stablecoin", async () => {
    // Find PDA for state
    [statePda, stateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [stateSeed],
      program.programId
    );

    await program.methods
      .initialize()
      .accounts({
        state: statePda,
        irmaAdmin: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        // clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .rpc();

    console.log("IRMA state initialized with PDA:", statePda.toBase58());

    // Fetch the state account and check values
    const stateAccount = await program.account.stateMap.fetch(statePda);
    assert.isDefined(stateAccount);
    // Add more assertions as needed
  });

  it("Adds a stablecoin", async () => {
    const symbol = "USDC";
    const mintAddress = anchor.web3.Keypair.generate().publicKey;
    const decimals = 6;

    console.log("Adding stablecoin:", symbol, "Mint Address:", mintAddress.toBase58(), "Decimals:", decimals);

    await program.methods
      .addStablecoin(symbol, mintAddress, decimals)
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
  });

  it("Call Crank", async () => {
    console.log("Calling Crank...");
    // Find PDA for state
    [crankPda, stateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [crankSeed],
      program.programId
    );
    console.log("Crank PDA:", crankPda.toJSON());

    await program.methods
      .crank()
      .accounts({
        state: crankPda,
        irmaAdmin: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Crank called");

    // Fetch and check state changes as needed
    const stateAccount = await program.account.stateMap.fetch(crankPda);
    assert.isDefined(stateAccount);
    // Add more assertions as needed
  });

  // Add more tests for redeem, mint, crank, etc.
});

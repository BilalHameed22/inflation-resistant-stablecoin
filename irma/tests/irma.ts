import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Irma } from "../target/types/irma";
import { assert } from "chai";

describe("irma", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  console.log("Using provider:", provider.wallet.publicKey.toBase58());
  const program = anchor.workspace.irma as Program<Irma>;
  if (!program) throw new Error("Program 'irma' not found in anchor.workspace");

  // Example keypairs
  const irmaAdmin = anchor.web3.Keypair.generate();
  const stateSeed = Buffer.from("state");
  let statePda: anchor.web3.PublicKey;
  let stateBump: number;

  before(async () => {
    // Find PDA for state
    [statePda, stateBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [stateSeed],
      program.programId
    );

    // Airdrop SOL to irmaAdmin for testing
    const sig = await provider.connection.requestAirdrop(
      irmaAdmin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    const config = "confirmed" as anchor.web3.Commitment;
    await provider.connection.confirmTransaction(sig, config);
  });

  it("Initializes IRMA state", async () => {
    await program.methods
      .initialize()
      .accounts({
        state: statePda,
        irmaAdmin: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([irmaAdmin])
      .rpc();

    // Fetch the state account and check values
    const stateAccount = await program.account.stateMap.fetch(statePda);
    assert.isDefined(stateAccount);
    // Add more assertions as needed
  });

  it("Adds a stablecoin", async () => {
    const symbol = "USDC";
    const mintAddress = anchor.web3.Keypair.generate().publicKey;
    const decimals = 6;

    await program.methods
      .addStablecoin(symbol, mintAddress, decimals)
      .accounts({
        state: statePda,
        irmaAdmin: irmaAdmin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([irmaAdmin])
      .rpc();

    // Fetch and check state changes as needed
    const stateAccount = await program.account.stateMap.fetch(statePda);
    assert.isDefined(stateAccount);
    // Add more assertions as needed
  });

  // Add more tests for redeem, mint, crank, etc.
});

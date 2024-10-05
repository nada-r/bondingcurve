import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { BondingCurve } from "../target/types/bonding_curve";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

describe("bonding-curve", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider();
  const connection = provider.connection;
  const program = anchor.workspace.BondingCurve as Program<BondingCurve>;

  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
  };

  const caller = Keypair.generate();

  it("Airdrop to caller", async () => {
    const balanceBefore = await connection.getBalance(caller.publicKey);

    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: caller.publicKey,
        lamports: 10 * LAMPORTS_PER_SOL,
      })
    );

    await provider.sendAndConfirm(tx, []).then(log);

    const balanceAfter = await connection.getBalance(caller.publicKey);
    // expect(balanceAfter).to.greaterThan(balanceBefore);
  });

  it("Create Caller", async () => {
    const tokenName = "Test Token";
    const tokenSymbol = "TEST";
    const uri = "https://example.com/token-metadata";

    const [callerAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("caller"), caller.publicKey.toBuffer()],
      program.programId
    );

    const [mint] = PublicKey.findProgramAddressSync(
      [Buffer.from("mint"), caller.publicKey.toBuffer()],
      program.programId
    );

    const [mintVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("mint_vault"), caller.publicKey.toBuffer()],
      program.programId
    );

    const [solVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("sol_vault"), caller.publicKey.toBuffer()],
      program.programId
    );

    const metadataAddress = PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
      TOKEN_PROGRAM_ID
    )[0];

    // Create associated token account for mintVault
    // console.log("Creating mint vault ATA");
    // const mintVaultATA = await getAssociatedTokenAddress(
    //   mint,
    //   caller.publicKey,
    //   false,
    //   TOKEN_PROGRAM_ID,
    // );

    // const createMintVaultATAIx = createAssociatedTokenAccountInstruction(
    //   provider.publicKey,
    //   mintVaultATA,
    //   caller.publicKey,
    //   mint,
    //   TOKEN_PROGRAM_ID
    // );

    // const tx = new Transaction().add(createMintVaultATAIx);
    // await provider.sendAndConfirm(tx);
    console.log("AZERTY", caller.publicKey.toBase58());

    console.log("Create caller");
    try {
      await program.methods
        .createCaller(tokenName, tokenSymbol, uri)
        .accountsPartial({
          caller: caller.publicKey,
          callerAccount,
          mint,
          metadataAccount: metadataAddress,
          mintVault,
          solVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenMetadataProgram: new PublicKey(
            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
          ), // Metaplex Token Metadata Program ID
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([caller])
        .rpc()
        .then(confirm)
        .then(log);
    } catch (e) {
      console.error("Error creating caller:", e);
      throw e;
    }
  });
});

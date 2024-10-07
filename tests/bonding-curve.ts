// Import necessary modules and dependencies
import assert from "assert";

import { BN } from "bn.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Metaplex } from "@metaplex-foundation/js";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

import {
  getAssociatedTokenAddress,
  getMint,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";

import { calculateFee } from "../client";
import { BondingCurve } from "../target/types/bonding_curve";

import {
  ammFromBondingCurve,
  fundAccountSOL,
  getAnchorError,
  getSPLBalance,
  sendTransaction,
  toEvent,
} from "./util";

// Define constants for program seeds
const CONFIG_SEED = "config";
const BONDING_CURVE_SEED = "bonding-curve";

// Test suite for Bonding Curve
describe("Bonding Curve", () => {
  // Define default values and constants
  const DEFAULT_DECIMALS = 6n;
  const DEFAULT_TOKEN_BALANCE =
    1_000_000_000n * BigInt(10 ** Number(DEFAULT_DECIMALS));
  const DEFAULT_INITIAL_TOKEN_RESERVES = 1_862_100_000_000_000n;
  const DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE = 30_000_000_000n;
  const DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE = 1_073_000_000_000_000n;
  const DEFAULT_FEE_BASIS_POINTS = 50n;

  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Initialize program and connection
  const program = anchor.workspace.BondingCurve as Program<BondingCurve>;
  const connection = provider.connection;

  // Generate keypairs for various roles
  const admin = anchor.web3.Keypair.generate();
  const caller = anchor.web3.Keypair.generate();
  const feeRecipient = anchor.web3.Keypair.generate();
  const withdrawAuthority = anchor.web3.Keypair.generate();
  const mint = anchor.web3.Keypair.generate();

  // Derive PDAs for config and bonding curve accounts
  const [configPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(CONFIG_SEED)],
    program.programId
  );
  const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
    program.programId
  );

  // Helper function to get AMM from bonding curve
  const getAmmFromBondingCurve = async () => {
    let bondingCurveAccount =
      await program.account.bondingCurve.fetch(bondingCurvePDA);
    return ammFromBondingCurve(
      bondingCurveAccount,
      DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE
    );
  };

  // Helper function to assert bonding curve state
  const assertBondingCurve = (
    amm: any,
    bondingCurveAccount: any,
    complete: boolean = false
  ) => {
    assert.equal(
      bondingCurveAccount.virtualTokenReserves.toString(),
      amm.virtualTokenReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.virtualSolReserves.toString(),
      amm.virtualSolReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.realTokenReserves.toString(),
      amm.realTokenReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.realSolReserves.toString(),
      amm.realSolReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.tokenTotalSupply.toString(),
      DEFAULT_TOKEN_BALANCE.toString()
    );
    assert.equal(bondingCurveAccount.complete, complete);
  };

  // Helper function to perform a simple buy transaction
  const simpleBuy = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    maxSolAmount: bigint,
    innerFeeRecipient: anchor.web3.Keypair = feeRecipient
  ) => {
    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      user,
      mint.publicKey,
      user.publicKey
    );

    let tx = await program.methods
      .buy(new BN(tokenAmount.toString()), new BN(maxSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        feeRecipient: innerFeeRecipient.publicKey,
        program: program.programId,
      })
      .transaction();

    let txResults = await sendTransaction(program, tx, [user], user.publicKey);

    return {
      tx: txResults,
      userTokenAccount,
      bondingCurveTokenAccount,
      bondingCurvePDA,
    };
  };

  // Helper function to perform a simple sell transaction
  const simpleSell = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    minSolAmount: bigint,
    innerFeeRecipient: anchor.web3.Keypair = feeRecipient
  ) => {
    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      user,
      mint.publicKey,
      user.publicKey
    );

    let tx = await program.methods
      .sell(new BN(tokenAmount.toString()), new BN(minSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        feeRecipient: innerFeeRecipient.publicKey,
        program: program.programId,
      })
      .transaction();

    let txResults = await sendTransaction(program, tx, [user], user.publicKey);

    return {
      tx: txResults,
      userTokenAccount,
      bondingCurveTokenAccount,
      bondingCurvePDA,
    };
  };

  // Before hook to fund accounts with SOL
  before(async () => {
    await fundAccountSOL(connection, admin.publicKey, 5 * LAMPORTS_PER_SOL);
    await fundAccountSOL(connection, caller.publicKey, 200 * LAMPORTS_PER_SOL);
    await fundAccountSOL(
      connection,
      withdrawAuthority.publicKey,
      5 * LAMPORTS_PER_SOL
    );
  });

  // Test case: Initialize the program and set parameters
  it("Should initialize the program and set parameters", async () => {
    // Initialize the program
    await program.methods
      .initialize(
        feeRecipient.publicKey,
        withdrawAuthority.publicKey,
        new BN(DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()),
        new BN(DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE.toString()),
        new BN(DEFAULT_INITIAL_TOKEN_RESERVES.toString()),
        new BN(DEFAULT_TOKEN_BALANCE.toString()),
        new BN(DEFAULT_FEE_BASIS_POINTS.toString())
      )
      .accounts({
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();
    let config = await program.account.config.fetch(configPDA);

    // Assert config account state
    assert.equal(config.authority.toBase58(), admin.publicKey.toBase58());
    assert.equal(config.initialized, true);
  });

  // Test case: Create a caller bonding curve
  it("Should create a caller bonding curve", async () => {
    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    let name = "Test";
    let symbol = "TEST";
    let uri = "https://www.test.com";

    // Create bonding curve transaction
    const tx = await program.methods
      .create(name, symbol, uri)
      .accounts({
        mint: mint.publicKey,
        creator: caller.publicKey,
        program: program.programId,
      })
      .transaction();

    let txResult = await sendTransaction(
      program,
      tx,
      [mint, caller],
      caller.publicKey
    );

    // Assert token account balance
    const tokenAmount = await connection.getTokenAccountBalance(
      bondingCurveTokenAccount
    );
    assert.equal(tokenAmount.value.amount, DEFAULT_TOKEN_BALANCE.toString());

    // Assert mint account state
    const createdMint = await getMint(connection, mint.publicKey);
    assert.equal(createdMint.isInitialized, true);
    assert.equal(createdMint.decimals, Number(DEFAULT_DECIMALS));
    assert.equal(createdMint.supply, DEFAULT_TOKEN_BALANCE);
    assert.equal(createdMint.mintAuthority, null);

    // Assert token metadata
    const metaplex = Metaplex.make(connection);
    const token = await metaplex
      .nfts()
      .findByMint({ mintAddress: mint.publicKey });
    assert.equal(token.name, name);
    assert.equal(token.symbol, symbol);
    assert.equal(token.uri, uri);

    // Assert bonding curve token account balance
    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      bondingCurveTokenAccount
    );
    assert.equal(
      bondingCurveTokenAccountInfo.value.amount,
      DEFAULT_TOKEN_BALANCE.toString()
    );

    // Assert bonding curve account state
    let bondingCurveAccount =
      await program.account.bondingCurve.fetch(bondingCurvePDA);
    assert.equal(
      bondingCurveAccount.virtualTokenReserves.toString(),
      DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()
    );
    assert.equal(
      bondingCurveAccount.virtualSolReserves.toString(),
      DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE.toString()
    );
    assert.equal(
      bondingCurveAccount.realTokenReserves.toString(),
      DEFAULT_INITIAL_TOKEN_RESERVES.toString()
    );
    assert.equal(bondingCurveAccount.realSolReserves.toString(), "0");
    assert.equal(
      bondingCurveAccount.tokenTotalSupply.toString(),
      DEFAULT_TOKEN_BALANCE.toString()
    );
    assert.equal(bondingCurveAccount.complete, false);
  });

  // Test case: Allow a user to buy caller tokens
  it("Should allow a user to buy caller tokens", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let buyTokenAmount = DEFAULT_TOKEN_BALANCE / 100n;
    let buyMaxSOLAmount = currentAMM.getBuyPrice(buyTokenAmount);
    let fee = calculateFee(buyMaxSOLAmount, Number(DEFAULT_FEE_BASIS_POINTS));
    buyMaxSOLAmount = buyMaxSOLAmount + fee;

    let buyResult = currentAMM.applyBuy(buyTokenAmount);

    // Check fee recipient balance before buy
    let feeRecipientPreBuySOLBalance = await connection.getBalance(
      feeRecipient.publicKey
    );

    // Perform buy transaction
    let txResult = await simpleBuy(caller, buyTokenAmount, buyMaxSOLAmount);

    // Check fee recipient balance after buy
    let feeRecipientPostBuySOLBalance = await connection.getBalance(
      feeRecipient.publicKey
    );
    assert.equal(
      feeRecipientPostBuySOLBalance - feeRecipientPreBuySOLBalance,
      Number(fee)
    );

    let targetCurrentSupply = (
      DEFAULT_TOKEN_BALANCE - buyTokenAmount
    ).toString();

    // Assert user token balance
    const tokenAmount = await connection.getTokenAccountBalance(
      txResult.userTokenAccount.address
    );
    assert.equal(tokenAmount.value.amount, buyTokenAmount.toString());

    // Assert bonding curve token account balance
    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      txResult.bondingCurveTokenAccount
    );
    assert.equal(
      bondingCurveTokenAccountInfo.value.amount,
      targetCurrentSupply
    );

    // Assert bonding curve account state
    let bondingCurveAccount =
      await program.account.bondingCurve.fetch(bondingCurvePDA);
    assertBondingCurve(currentAMM, bondingCurveAccount);
  });

  // Test case: Allow a user to sell caller tokens
  it("Should allow a user to sell caller tokens", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let tokenAmount = 10000000n;
    let minSolAmount = currentAMM.getSellPrice(tokenAmount);
    let fee = calculateFee(minSolAmount, Number(DEFAULT_FEE_BASIS_POINTS));
    minSolAmount = minSolAmount - fee;

    let sellResults = currentAMM.applySell(tokenAmount);

    // Check balances before sale
    let userPreSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      caller.publicKey
    );
    let curvePreSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      bondingCurvePDA,
      true
    );
    let feeRecipientPreBuySOLBalance = await connection.getBalance(
      feeRecipient.publicKey
    );

    // Perform sell transaction
    let txResult = await simpleSell(caller, tokenAmount, minSolAmount);

    // Check fee recipient balance after sale
    let feeRecipientPostBuySOLBalance = await connection.getBalance(
      feeRecipient.publicKey
    );
    assert.equal(
      feeRecipientPostBuySOLBalance - feeRecipientPreBuySOLBalance,
      Number(fee)
    );

    // Assert bonding curve token account balance after sale
    let curvePostSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      bondingCurvePDA,
      true
    );
    assert.equal(
      curvePostSaleBalance,
      (BigInt(curvePreSaleBalance) + tokenAmount).toString()
    );

    // Assert bonding curve account state
    let bondingCurveAccount =
      await program.account.bondingCurve.fetch(bondingCurvePDA);
    assertBondingCurve(currentAMM, bondingCurveAccount);
  });

  // Test case: Prevent withdrawal from curve when incomplete
  it("Should prevent withdrawal from curve when incomplete", async () => {
    let errorCode = "";
    try {
      let tx = await program.methods
        .withdraw()
        .accounts({
          user: withdrawAuthority.publicKey,
          mint: mint.publicKey,
        })
        .transaction();

      await sendTransaction(
        program,
        tx,
        [withdrawAuthority],
        withdrawAuthority.publicKey
      );
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "BondingCurveNotComplete");
  });
});

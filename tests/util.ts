// Import necessary modules and types
import * as anchor from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SendTransactionError,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";

import { BondingCurve } from "../target/types/bonding_curve";
import * as client from "../client";

// Build a versioned transaction
export const buildVersionedTx = async (
  connection: anchor.web3.Connection,
  payer: PublicKey,
  tx: Transaction
) => {
  const blockHash = (await connection.getLatestBlockhash("processed"))
    .blockhash;

  let messageV0 = new TransactionMessage({
    payerKey: payer,
    recentBlockhash: blockHash,
    instructions: tx.instructions,
  }).compileToV0Message();

  return new VersionedTransaction(messageV0);
};

// Get transaction details
export const getTxDetails = async (connection: anchor.web3.Connection, sig) => {
  const latestBlockHash = await connection.getLatestBlockhash("processed");

  await connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: sig,
    },
    "confirmed"
  );

  return await connection.getTransaction(sig, {
    maxSupportedTransactionVersion: 0,
    commitment: "confirmed",
  });
};

// Send a transaction
export const sendTransaction = async (
  program: anchor.Program<BondingCurve>,
  tx: Transaction,
  signers: anchor.web3.Signer[],
  payer: PublicKey
) => {
  const versionedTx = await buildVersionedTx(
    program.provider.connection,
    payer,
    tx
  );
  versionedTx.sign(signers);

  let sig = await program.provider.connection.sendTransaction(versionedTx);
  let response = await getTxDetails(program.provider.connection, sig);

  return response;
};

// Get Anchor error
export const getAnchorError = (error: any) => {
  if (error instanceof anchor.AnchorError) {
    return error;
  } else if (error instanceof SendTransactionError) {
    return anchor.AnchorError.parse(error.logs || []);
  }
  return null;
};

// Fund an account with SOL
export const fundAccountSOL = async (
  connection: anchor.web3.Connection,
  publicKey: anchor.web3.PublicKey,
  amount: number
) => {
  let fundSig = await connection.requestAirdrop(publicKey, amount);

  return getTxDetails(connection, fundSig);
};

// Create AMM from bonding curve account
export const ammFromBondingCurve = (
  bondingCurveAccount: anchor.IdlAccounts<BondingCurve>["bondingCurve"] | null,
  initialVirtualTokenReserves: bigint
) => {
  if (!bondingCurveAccount) throw new Error("Bonding curve account not found");
  return new client.AMM(
    BigInt(bondingCurveAccount.virtualSolReserves.toString()),
    BigInt(bondingCurveAccount.virtualTokenReserves.toString()),
    BigInt(bondingCurveAccount.realSolReserves.toString()),
    BigInt(bondingCurveAccount.realTokenReserves.toString()),
    initialVirtualTokenReserves
  );
};

// Convert bigint to SOL
export const bigIntToSOL = (amount: bigint) => {
  return amount / BigInt(LAMPORTS_PER_SOL);
};

// Get SPL token balance
export const getSPLBalance = async (
  connection: Connection,
  mintAddress: PublicKey,
  pubKey: PublicKey,
  allowOffCurve: boolean = false
) => {
  try {
    let ata = getAssociatedTokenAddressSync(mintAddress, pubKey, allowOffCurve);
    const balance = await connection.getTokenAccountBalance(ata, "processed");
    return balance.value.amount;
  } catch (e) {
    console.error(e);
  }
  return "0";
};

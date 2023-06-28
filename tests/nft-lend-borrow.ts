import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    createAccount,
    createMint,
    getAccount,
    mintTo,
} from "@solana/spl-token";
import { NftLendBorrow } from "../target/types/nft_lend_borrow";
import { assert } from "chai";

describe("nft-lend-borrow", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.local();
    anchor.setProvider(provider);

    const program = anchor.workspace.NftLendBorrow as Program<NftLendBorrow>;

    let assetMint: PublicKey;

    let lenderAssetAccount: PublicKey;
    let borrowerAssetAccount: PublicKey;
    let vaultAssetAccount: PublicKey;

    let payer = anchor.web3.Keypair.generate();
    let mintAuthority = anchor.web3.Keypair.generate();
    let assetPoolAuthority = anchor.web3.Keypair.generate();

    let lender = anchor.web3.Keypair.generate();
    let borrower = anchor.web3.Keypair.generate();

    let collectionPool: PublicKey;

    let collectionId = new PublicKey(
        "J1S9H3QjnRtBbbuD4HjPV6RpRhwuk4zKbxsnCHuTgh9w"
    );

    let loanDuration = 30;

    it("Can initialize the state of the world", async () => {
        const transferSig = await provider.connection.requestAirdrop(
            payer.publicKey,
            20000000000
        );

        const latestBlockHash = await provider.connection.getLatestBlockhash();

        await provider.connection.confirmTransaction({
            blockhash: latestBlockHash.blockhash,
            lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
            signature: transferSig,
        });

        const tx = new Transaction();

        tx.add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mintAuthority.publicKey,
                lamports: 1000000000,
            }),
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: assetPoolAuthority.publicKey,
                lamports: 1000000000,
            }),
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: lender.publicKey,
                lamports: 15000000000,
            }),
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: borrower.publicKey,
                lamports: 2000000000,
            })
        );

        await provider.sendAndConfirm(tx, [payer]);

        assetMint = await createMint(
            provider.connection,
            payer,
            mintAuthority.publicKey,
            undefined,
            0,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID
        );

        lenderAssetAccount = await createAccount(
            provider.connection,
            payer,
            assetMint,
            lender.publicKey,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID
        );

        borrowerAssetAccount = await createAccount(
            provider.connection,
            payer,
            assetMint,
            borrower.publicKey,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID
        );

        await mintTo(
            provider.connection,
            payer,
            assetMint,
            borrowerAssetAccount,
            mintAuthority,
            1
        );

        let [collectionPoolAddress, _collectionBump] =
            anchor.web3.PublicKey.findProgramAddressSync(
                [Buffer.from("collection_pool"), collectionId.toBuffer()],
                program.programId
            );

        collectionPool = collectionPoolAddress;

        const borrowerAssetTokenAccount = await getAccount(
            provider.connection,
            borrowerAssetAccount
        );

        assert.strictEqual(borrowerAssetTokenAccount.amount.toString(), "1");
    });

    it("Can create pool", async () => {
        await program.methods
            .createPool(collectionId, new anchor.BN(loanDuration))
            .accounts({
                collectionPool: collectionPool,
                authority: assetPoolAuthority.publicKey,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([assetPoolAuthority])
            .rpc();

        const createdPool = await program.account.collectionPool.fetch(
            collectionPool
        );

        assert.strictEqual(
            createdPool.collectionId.toBase58(),
            collectionId.toBase58()
        );
        assert.strictEqual(createdPool.duration.toNumber(), loanDuration);
        assert.strictEqual(
            createdPool.poolOwner.toBase58(),
            assetPoolAuthority.publicKey.toBase58()
        );
    });
});

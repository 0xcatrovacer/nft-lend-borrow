import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
} from "@solana/web3.js";
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
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.NftLendBorrow as Program<NftLendBorrow>;

    let assetMint: PublicKey;

    let lenderAssetAccount: PublicKey;
    let borrowerAssetAccount: PublicKey;
    let vaultAssetAccount: PublicKey;

    let payer = anchor.web3.Keypair.generate();
    let mintAuthority = anchor.web3.Keypair.generate();
    let assetPoolAuthority = anchor.web3.Keypair.generate();
    let vault = anchor.web3.Keypair.generate();

    let lender = anchor.web3.Keypair.generate();
    let borrower = anchor.web3.Keypair.generate();

    let lenderInitialBalance = 10000000000;
    let borrowerInitialBalance = 5000000000;

    let collectionPoolPDA: PublicKey;
    let offerPDA: PublicKey;
    let activeLoanPDA: PublicKey;

    let collectionId = new PublicKey(
        "J1S9H3QjnRtBbbuD4HjPV6RpRhwuk4zKbxsnCHuTgh9w"
    );

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
                lamports: lenderInitialBalance,
            }),
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: borrower.publicKey,
                lamports: borrowerInitialBalance,
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
                [
                    anchor.utils.bytes.utf8.encode("collection-pool"),
                    collectionId.toBuffer(),
                ],
                program.programId
            );

        collectionPoolPDA = collectionPoolAddress;

        const borrowerAssetTokenAccount = await getAccount(
            provider.connection,
            borrowerAssetAccount
        );

        assert.strictEqual(borrowerAssetTokenAccount.amount.toString(), "1");
    });

    let loanDuration = 30;

    it("Can create pool", async () => {
        await program.methods
            .createPool(collectionId, new anchor.BN(loanDuration))
            .accounts({
                collectionPool: collectionPoolPDA,
                authority: assetPoolAuthority.publicKey,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([assetPoolAuthority])
            .rpc();

        const createdPool = await program.account.collectionPool.fetch(
            collectionPoolPDA
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

    let totalOffers = 0;
    let offerAmount = new anchor.BN(2 * LAMPORTS_PER_SOL);

    it("Can offer loan", async () => {
        let [offer, _offerBump] = anchor.web3.PublicKey.findProgramAddressSync(
            [
                anchor.utils.bytes.utf8.encode("offer"),
                collectionPoolPDA.toBuffer(),
                lender.publicKey.toBuffer(),
                Buffer.from(totalOffers.toString()),
            ],
            program.programId
        );

        offerPDA = offer;

        await program.methods
            .offerLoan(offerAmount)
            .accounts({
                offerLoan: offerPDA,
                vaultAccount: vault.publicKey,
                collectionPool: collectionPoolPDA,
                lender: lender.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([lender, vault])
            .rpc();

        const vaultAccount = await provider.connection.getAccountInfo(
            vault.publicKey
        );
        const lenderAccount = await provider.connection.getAccountInfo(
            lender.publicKey
        );

        assert.isAbove(vaultAccount.lamports, offerAmount.toNumber());
        assert.isBelow(
            lenderAccount.lamports,
            lenderInitialBalance - offerAmount.toNumber()
        );

        const createdOffer = await program.account.offer.fetch(offerPDA);

        assert.strictEqual(
            createdOffer.collection.toBase58(),
            collectionPoolPDA.toBase58()
        );
        assert.strictEqual(
            createdOffer.offerLamportAmount.toNumber(),
            offerAmount.toNumber()
        );
        assert.strictEqual(
            createdOffer.repayLamportAmount.toNumber(),
            offerAmount.toNumber() + (10 / 100) * offerAmount.toNumber()
        );
        assert.strictEqual(
            createdOffer.lender.toBase58(),
            lender.publicKey.toBase58()
        );
        assert.strictEqual(createdOffer.isLoanTaken, false);
    });
});

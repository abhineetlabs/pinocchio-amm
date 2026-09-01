import * as path from 'node:path';
import {
    type Address,
    appendTransactionMessageInstructions,
    createTransactionMessage,
    generateKeyPairSigner,
    getAddressEncoder,
    getProgramDerivedAddress,
    getU32Encoder,
    type Instruction,
    type KeyPairSigner,
    lamports,
    pipe,
    setTransactionMessageFeePayerSigner,
    signTransactionMessageWithSigners,
} from '@solana/kit';
import { getCreateAccountInstruction, getTransferSolInstruction } from '@solana-program/system';
import {
    findAssociatedTokenPda,
    getCreateAssociatedTokenIdempotentInstruction,
    getInitializeMint2Instruction,
    getMintSize,
    getMintToInstruction,
    TOKEN_PROGRAM_ADDRESS,
} from '@solana-program/token';
import { FailedTransactionMetadata, LiteSVM } from 'litesvm';
import { buildInitializeAmm, buildInitializePool } from './instruction';

const addressEncoder = getAddressEncoder();

export const PROGRAM_ID = 'GGZzCxQb9D7v84Ai1WkQgeqRx79j8pRZfk8yQmF3Jvqo' as Address;
export const PROGRAM_SO = path.join(process.cwd(), 'tests', 'fixtures', 'pinocchio_amm.so');

export async function sendInstructions(svm: LiteSVM, payer: KeyPairSigner, instructions: readonly Instruction[]) {
    const transactionMessage = pipe(
        createTransactionMessage({ version: 0 }),
        message => setTransactionMessageFeePayerSigner(payer, message),
        message => svm.setTransactionMessageLifetimeUsingLatestBlockhash(message),
        message => appendTransactionMessageInstructions(instructions, message),
    );
    const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);
    const result = svm.sendTransaction(signedTransaction);
    if (result instanceof FailedTransactionMetadata) {
        throw new Error(`transaction failed: ${result.toString()}`);
    }
}

export async function expectFailure(promise: Promise<unknown>) {
    try {
        await promise;
    } catch {
        return;
    }
    throw new Error('Expected transaction to fail');
}

function compareAddresses(left: Address, right: Address) {
    const leftBytes = addressEncoder.encode(left);
    const rightBytes = addressEncoder.encode(right);
    for (let index = 0; index < leftBytes.length; index += 1) {
        if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] - rightBytes[index];
    }
    return 0;
}

async function createMint(svm: LiteSVM, payer: KeyPairSigner, mint: KeyPairSigner) {
    const mintSize = getMintSize();
    await sendInstructions(svm, payer, [
        getCreateAccountInstruction({
            payer,
            newAccount: mint,
            lamports: svm.minimumBalanceForRentExemption(BigInt(mintSize)),
            space: mintSize,
            programAddress: TOKEN_PROGRAM_ADDRESS,
        }),
        getInitializeMint2Instruction({
            mint: mint.address,
            decimals: 6,
            mintAuthority: payer.address,
            freezeAuthority: null,
        }),
    ]);
}

async function createAndFundAta(svm: LiteSVM, payer: KeyPairSigner, owner: Address, mint: Address, amount: bigint) {
    const [ata] = await findAssociatedTokenPda({ mint, owner, tokenProgram: TOKEN_PROGRAM_ADDRESS });
    await sendInstructions(svm, payer, [
        getCreateAssociatedTokenIdempotentInstruction({ payer, ata, owner, mint }),
        getMintToInstruction({ mint, token: ata, mintAuthority: payer, amount }),
    ]);
    return ata;
}

export interface TestContext {
    svm: LiteSVM;
    payer: KeyPairSigner;
    admin: KeyPairSigner;
    newAdmin: KeyPairSigner;
    trader: KeyPairSigner;
    id: number;
    fee: number;
    amm: Address;
    pool: Address;
    mintA: KeyPairSigner;
    mintB: KeyPairSigner;
    mintLp: Address;
    poolAtaA: Address;
    poolAtaB: Address;
    adminAtaA: Address;
    adminAtaB: Address;
    adminAtaLp: Address;
    traderAtaA: Address;
    traderAtaB: Address;
}

export async function setupPool(options?: { prefundAmm?: boolean; fee?: number }): Promise<TestContext> {
    const svm = new LiteSVM();
    svm.addProgramFromFile(PROGRAM_ID, PROGRAM_SO);

    const payer = await generateKeyPairSigner();
    const admin = await generateKeyPairSigner();
    const newAdmin = await generateKeyPairSigner();
    const trader = await generateKeyPairSigner();
    svm.airdrop(payer.address, lamports(20_000_000_000n));

    let mintA = await generateKeyPairSigner();
    let mintB = await generateKeyPairSigner();
    if (compareAddresses(mintA.address, mintB.address) > 0) {
        [mintA, mintB] = [mintB, mintA];
    }

    await createMint(svm, payer, mintA);
    await createMint(svm, payer, mintB);

    const fundedAmount = 100_000_000n;
    const adminAtaA = await createAndFundAta(svm, payer, admin.address, mintA.address, fundedAmount);
    const adminAtaB = await createAndFundAta(svm, payer, admin.address, mintB.address, fundedAmount);
    const traderAtaA = await createAndFundAta(svm, payer, trader.address, mintA.address, fundedAmount);
    const traderAtaB = await createAndFundAta(svm, payer, trader.address, mintB.address, fundedAmount);

    const id = 42;
    const fee = options?.fee ?? 500;
    const [amm] = await getProgramDerivedAddress({
        programAddress: PROGRAM_ID,
        seeds: ['amm', addressEncoder.encode(admin.address), getU32Encoder().encode(id)],
    });
    const [pool] = await getProgramDerivedAddress({
        programAddress: PROGRAM_ID,
        seeds: [addressEncoder.encode(amm), addressEncoder.encode(mintA.address), addressEncoder.encode(mintB.address)],
    });
    const [mintLp] = await getProgramDerivedAddress({
        programAddress: PROGRAM_ID,
        seeds: [
            addressEncoder.encode(amm),
            addressEncoder.encode(mintA.address),
            addressEncoder.encode(mintB.address),
            'liquidity',
        ],
    });
    const [poolAtaA] = await findAssociatedTokenPda({
        mint: mintA.address,
        owner: pool,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    const [poolAtaB] = await findAssociatedTokenPda({
        mint: mintB.address,
        owner: pool,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    const [adminAtaLp] = await findAssociatedTokenPda({
        mint: mintLp,
        owner: admin.address,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });

    if (options?.prefundAmm) {
        await sendInstructions(svm, payer, [
            getTransferSolInstruction({ source: payer, destination: amm, amount: 1_000_000 }),
        ]);
    }

    await sendInstructions(svm, payer, [buildInitializeAmm({ payer, admin, amm, id, fee, programId: PROGRAM_ID })]);
    await sendInstructions(svm, payer, [
        buildInitializePool({
            payer,
            amm,
            pool,
            mintLp,
            mintA: mintA.address,
            mintB: mintB.address,
            poolAtaA,
            poolAtaB,
            programId: PROGRAM_ID,
        }),
    ]);

    return {
        svm,
        payer,
        admin,
        newAdmin,
        trader,
        id,
        fee,
        amm,
        pool,
        mintA,
        mintB,
        mintLp,
        poolAtaA,
        poolAtaB,
        adminAtaA,
        adminAtaB,
        adminAtaLp,
        traderAtaA,
        traderAtaB,
    };
}

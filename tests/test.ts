import { type Address, getAddressEncoder, getProgramDerivedAddress, getU32Encoder } from '@solana/kit';
import { getMintDecoder, getTokenDecoder } from '@solana-program/token';
import { assert } from 'chai';
import { ammDecoder, poolDecoder } from './account';
import {
    buildDeposit,
    buildInitializeAmm,
    buildSetPaused,
    buildSwap,
    buildTransferAdmin,
    buildUpdateFee,
    buildWithdraw,
} from './instruction';
import { expectFailure, PROGRAM_ID, sendInstructions, setupPool, type TestContext } from './utils';

const addressEncoder = getAddressEncoder();

function accountData(context: TestContext, account: Address) {
    const info = context.svm.getAccount(account);
    if (!info.exists) throw new Error(`Account not found: ${account}`);
    return info.data;
}

function tokenAmount(context: TestContext, account: Address) {
    return getTokenDecoder().decode(accountData(context, account)).amount;
}

function mintSupply(context: TestContext) {
    return getMintDecoder().decode(accountData(context, context.mintLp)).supply;
}

function initialDeposit(context: TestContext) {
    return buildDeposit({
        amm: context.amm,
        pool: context.pool,
        depositor: context.admin,
        mintLp: context.mintLp,
        poolAtaA: context.poolAtaA,
        poolAtaB: context.poolAtaB,
        depositorAtaLp: context.adminAtaLp,
        depositorAtaA: context.adminAtaA,
        depositorAtaB: context.adminAtaB,
        payer: context.payer,
        amountA: 4_000_000n,
        amountB: 1_000_000n,
        programId: PROGRAM_ID,
    });
}

describe('Token swap (Pinocchio)', () => {
    it('initializes a prefunded AMM and pool', async () => {
        const context = await setupPool({ prefundAmm: true });
        const amm = ammDecoder.decode(accountData(context, context.amm));
        const pool = poolDecoder.decode(accountData(context, context.pool));

        assert.equal(amm.creator, context.admin.address);
        assert.equal(amm.admin, context.admin.address);
        assert.equal(amm.id, context.id);
        assert.equal(amm.fee, context.fee);
        assert.equal(amm.paused, 0);
        assert.equal(pool.amm, context.amm);
        assert.equal(pool.mintA, context.mintA.address);
        assert.equal(pool.mintB, context.mintB.address);
        assert.equal(tokenAmount(context, context.poolAtaA), 0n);
        assert.equal(tokenAmount(context, context.poolAtaB), 0n);
        assert.equal(mintSupply(context), 0n);
    });

    it('rejects an invalid fee', async () => {
        const context = await setupPool();
        const id = context.id + 1;
        const [amm] = await getProgramDerivedAddress({
            programAddress: PROGRAM_ID,
            seeds: ['amm', addressEncoder.encode(context.admin.address), getU32Encoder().encode(id)],
        });

        await expectFailure(
            sendInstructions(context.svm, context.payer, [
                buildInitializeAmm({
                    payer: context.payer,
                    admin: context.admin,
                    amm,
                    id,
                    fee: 10_000,
                    programId: PROGRAM_ID,
                }),
            ]),
        );
    });

    it('deposits, swaps, prices later LP shares, and withdraws', async () => {
        const context = await setupPool();
        await sendInstructions(context.svm, context.payer, [initialDeposit(context)]);

        assert.equal(tokenAmount(context, context.poolAtaA), 4_000_000n);
        assert.equal(tokenAmount(context, context.poolAtaB), 1_000_000n);
        assert.equal(tokenAmount(context, context.adminAtaLp), 1_999_900n);
        assert.equal(mintSupply(context), 1_999_900n);

        const input = 100_000n;
        const feeAmount = (input * BigInt(context.fee)) / 10_000n;
        const taxedInput = input - feeAmount;
        const output = (taxedInput * 1_000_000n) / (4_000_000n + taxedInput);
        const traderABefore = tokenAmount(context, context.traderAtaA);
        const traderBBefore = tokenAmount(context, context.traderAtaB);

        await sendInstructions(context.svm, context.payer, [
            buildSwap({
                amm: context.amm,
                pool: context.pool,
                trader: context.trader,
                mintA: context.mintA.address,
                mintB: context.mintB.address,
                poolAtaA: context.poolAtaA,
                poolAtaB: context.poolAtaB,
                traderAtaA: context.traderAtaA,
                traderAtaB: context.traderAtaB,
                payer: context.payer,
                swapA: true,
                inputAmount: input,
                minimumOutputAmount: output,
                programId: PROGRAM_ID,
            }),
        ]);

        const reserveA = 4_000_000n + input;
        const reserveB = 1_000_000n - output;
        assert.equal(tokenAmount(context, context.poolAtaA), reserveA);
        assert.equal(tokenAmount(context, context.poolAtaB), reserveB);
        assert.equal(tokenAmount(context, context.traderAtaA), traderABefore - input);
        assert.equal(tokenAmount(context, context.traderAtaB), traderBBefore + output);

        const requestedA = 410_000n;
        const requestedB = 100_000n;
        const depositedB = (requestedA * reserveB) / reserveA;
        const totalLiquidity = mintSupply(context) + 100n;
        const expectedLp = [(requestedA * totalLiquidity) / reserveA, (depositedB * totalLiquidity) / reserveB].reduce(
            (minimum, amount) => (amount < minimum ? amount : minimum),
        );
        const lpBefore = tokenAmount(context, context.adminAtaLp);

        await sendInstructions(context.svm, context.payer, [
            buildDeposit({
                amm: context.amm,
                pool: context.pool,
                depositor: context.admin,
                mintLp: context.mintLp,
                poolAtaA: context.poolAtaA,
                poolAtaB: context.poolAtaB,
                depositorAtaLp: context.adminAtaLp,
                depositorAtaA: context.adminAtaA,
                depositorAtaB: context.adminAtaB,
                payer: context.payer,
                amountA: requestedA,
                amountB: requestedB,
                programId: PROGRAM_ID,
            }),
        ]);

        assert.equal(tokenAmount(context, context.poolAtaA), reserveA + requestedA);
        assert.equal(tokenAmount(context, context.poolAtaB), reserveB + depositedB);
        assert.equal(tokenAmount(context, context.adminAtaLp), lpBefore + expectedLp);

        const burnAmount = 100_000n;
        const reserveABeforeWithdraw = tokenAmount(context, context.poolAtaA);
        const reserveBBeforeWithdraw = tokenAmount(context, context.poolAtaB);
        const supplyBeforeWithdraw = mintSupply(context);
        const withdrawA = (burnAmount * reserveABeforeWithdraw) / (supplyBeforeWithdraw + 100n);
        const withdrawB = (burnAmount * reserveBBeforeWithdraw) / (supplyBeforeWithdraw + 100n);

        await sendInstructions(context.svm, context.payer, [
            buildWithdraw({
                pool: context.pool,
                depositor: context.admin,
                mintLp: context.mintLp,
                mintA: context.mintA.address,
                mintB: context.mintB.address,
                poolAtaA: context.poolAtaA,
                poolAtaB: context.poolAtaB,
                depositorAtaLp: context.adminAtaLp,
                depositorAtaA: context.adminAtaA,
                depositorAtaB: context.adminAtaB,
                payer: context.payer,
                amount: burnAmount,
                programId: PROGRAM_ID,
            }),
        ]);

        assert.equal(mintSupply(context), supplyBeforeWithdraw - burnAmount);
        assert.equal(tokenAmount(context, context.poolAtaA), reserveABeforeWithdraw - withdrawA);
        assert.equal(tokenAmount(context, context.poolAtaB), reserveBBeforeWithdraw - withdrawB);
    });

    it('swaps from token B to token A', async () => {
        const context = await setupPool();
        await sendInstructions(context.svm, context.payer, [initialDeposit(context)]);

        const input = 50_000n;
        const taxedInput = input - (input * BigInt(context.fee)) / 10_000n;
        const output = (taxedInput * 4_000_000n) / (1_000_000n + taxedInput);

        await expectFailure(
            sendInstructions(context.svm, context.payer, [
                buildSwap({
                    amm: context.amm,
                    pool: context.pool,
                    trader: context.trader,
                    mintA: context.mintA.address,
                    mintB: context.mintB.address,
                    poolAtaA: context.poolAtaA,
                    poolAtaB: context.poolAtaB,
                    traderAtaA: context.traderAtaA,
                    traderAtaB: context.traderAtaB,
                    payer: context.payer,
                    swapA: false,
                    inputAmount: input,
                    minimumOutputAmount: output + 1n,
                    programId: PROGRAM_ID,
                }),
            ]),
        );
        assert.equal(tokenAmount(context, context.poolAtaA), 4_000_000n);
        assert.equal(tokenAmount(context, context.poolAtaB), 1_000_000n);

        await sendInstructions(context.svm, context.payer, [
            buildSwap({
                amm: context.amm,
                pool: context.pool,
                trader: context.trader,
                mintA: context.mintA.address,
                mintB: context.mintB.address,
                poolAtaA: context.poolAtaA,
                poolAtaB: context.poolAtaB,
                traderAtaA: context.traderAtaA,
                traderAtaB: context.traderAtaB,
                payer: context.payer,
                swapA: false,
                inputAmount: input,
                minimumOutputAmount: output,
                programId: PROGRAM_ID,
            }),
        ]);

        assert.equal(tokenAmount(context, context.poolAtaA), 4_000_000n - output);
        assert.equal(tokenAmount(context, context.poolAtaB), 1_000_000n + input);
    });

    it('enforces pause and admin authority', async () => {
        const context = await setupPool();
        await sendInstructions(context.svm, context.payer, [initialDeposit(context)]);

        await sendInstructions(context.svm, context.payer, [
            buildUpdateFee({ admin: context.admin, amm: context.amm, fee: 30, programId: PROGRAM_ID }),
            buildSetPaused({ admin: context.admin, amm: context.amm, paused: true, programId: PROGRAM_ID }),
        ]);
        let amm = ammDecoder.decode(accountData(context, context.amm));
        assert.equal(amm.fee, 30);
        assert.equal(amm.paused, 1);

        await expectFailure(
            sendInstructions(context.svm, context.payer, [
                buildSwap({
                    amm: context.amm,
                    pool: context.pool,
                    trader: context.trader,
                    mintA: context.mintA.address,
                    mintB: context.mintB.address,
                    poolAtaA: context.poolAtaA,
                    poolAtaB: context.poolAtaB,
                    traderAtaA: context.traderAtaA,
                    traderAtaB: context.traderAtaB,
                    payer: context.payer,
                    swapA: true,
                    inputAmount: 1_000n,
                    minimumOutputAmount: 1n,
                    programId: PROGRAM_ID,
                }),
            ]),
        );

        await sendInstructions(context.svm, context.payer, [
            buildTransferAdmin({
                admin: context.admin,
                newAdmin: context.newAdmin,
                amm: context.amm,
                programId: PROGRAM_ID,
            }),
        ]);
        await expectFailure(
            sendInstructions(context.svm, context.payer, [
                buildUpdateFee({ admin: context.admin, amm: context.amm, fee: 40, programId: PROGRAM_ID }),
            ]),
        );
        await sendInstructions(context.svm, context.payer, [
            buildSetPaused({ admin: context.newAdmin, amm: context.amm, paused: false, programId: PROGRAM_ID }),
            buildUpdateFee({ admin: context.newAdmin, amm: context.amm, fee: 40, programId: PROGRAM_ID }),
        ]);

        amm = ammDecoder.decode(accountData(context, context.amm));
        assert.equal(amm.admin, context.newAdmin.address);
        assert.equal(amm.fee, 40);
        assert.equal(amm.paused, 0);
    });
});

import {
    AccountRole,
    type Address,
    getStructEncoder,
    getU8Encoder,
    getU16Encoder,
    getU32Encoder,
    getU64Encoder,
    type KeyPairSigner,
} from '@solana/kit';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';
import { ASSOCIATED_TOKEN_PROGRAM_ADDRESS, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';

const initializeAmmEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['id', getU32Encoder()],
    ['fee', getU16Encoder()],
]);

const initializePoolEncoder = getStructEncoder([['instruction', getU8Encoder()]]);

const depositEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['amountA', getU64Encoder()],
    ['amountB', getU64Encoder()],
]);

const withdrawEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['amount', getU64Encoder()],
]);

const swapEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['swapA', getU8Encoder()],
    ['inputAmount', getU64Encoder()],
    ['minimumOutputAmount', getU64Encoder()],
]);

const updateFeeEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['fee', getU16Encoder()],
]);

const setPausedEncoder = getStructEncoder([
    ['instruction', getU8Encoder()],
    ['paused', getU8Encoder()],
]);

const transferAdminEncoder = getStructEncoder([['instruction', getU8Encoder()]]);

export function buildInitializeAmm(props: {
    payer: KeyPairSigner;
    admin: KeyPairSigner;
    amm: Address;
    id: number;
    fee: number;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.payer.address, role: AccountRole.WRITABLE_SIGNER, signer: props.payer },
            { address: props.admin.address, role: AccountRole.READONLY_SIGNER, signer: props.admin },
            { address: props.amm, role: AccountRole.WRITABLE },
            { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
        ],
        data: initializeAmmEncoder.encode({ instruction: 0, id: props.id, fee: props.fee }),
    };
}

export function buildInitializePool(props: {
    payer: KeyPairSigner;
    amm: Address;
    pool: Address;
    mintLp: Address;
    mintA: Address;
    mintB: Address;
    poolAtaA: Address;
    poolAtaB: Address;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.payer.address, role: AccountRole.WRITABLE_SIGNER, signer: props.payer },
            { address: props.amm, role: AccountRole.READONLY },
            { address: props.pool, role: AccountRole.WRITABLE },
            { address: props.mintLp, role: AccountRole.WRITABLE },
            { address: props.mintA, role: AccountRole.READONLY },
            { address: props.mintB, role: AccountRole.READONLY },
            { address: props.poolAtaA, role: AccountRole.WRITABLE },
            { address: props.poolAtaB, role: AccountRole.WRITABLE },
            { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
        ],
        data: initializePoolEncoder.encode({ instruction: 1 }),
    };
}

export function buildDeposit(props: {
    amm: Address;
    pool: Address;
    depositor: KeyPairSigner;
    mintLp: Address;
    poolAtaA: Address;
    poolAtaB: Address;
    depositorAtaLp: Address;
    depositorAtaA: Address;
    depositorAtaB: Address;
    payer: KeyPairSigner;
    amountA: bigint;
    amountB: bigint;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.amm, role: AccountRole.READONLY },
            { address: props.pool, role: AccountRole.READONLY },
            { address: props.depositor.address, role: AccountRole.READONLY_SIGNER, signer: props.depositor },
            { address: props.mintLp, role: AccountRole.WRITABLE },
            { address: props.poolAtaA, role: AccountRole.WRITABLE },
            { address: props.poolAtaB, role: AccountRole.WRITABLE },
            { address: props.depositorAtaLp, role: AccountRole.WRITABLE },
            { address: props.depositorAtaA, role: AccountRole.WRITABLE },
            { address: props.depositorAtaB, role: AccountRole.WRITABLE },
            { address: props.payer.address, role: AccountRole.WRITABLE_SIGNER, signer: props.payer },
            { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
        ],
        data: depositEncoder.encode({ instruction: 2, amountA: props.amountA, amountB: props.amountB }),
    };
}

export function buildWithdraw(props: {
    pool: Address;
    depositor: KeyPairSigner;
    mintLp: Address;
    mintA: Address;
    mintB: Address;
    poolAtaA: Address;
    poolAtaB: Address;
    depositorAtaLp: Address;
    depositorAtaA: Address;
    depositorAtaB: Address;
    payer: KeyPairSigner;
    amount: bigint;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.pool, role: AccountRole.READONLY },
            { address: props.depositor.address, role: AccountRole.READONLY_SIGNER, signer: props.depositor },
            { address: props.mintLp, role: AccountRole.WRITABLE },
            { address: props.mintA, role: AccountRole.READONLY },
            { address: props.mintB, role: AccountRole.READONLY },
            { address: props.poolAtaA, role: AccountRole.WRITABLE },
            { address: props.poolAtaB, role: AccountRole.WRITABLE },
            { address: props.depositorAtaLp, role: AccountRole.WRITABLE },
            { address: props.depositorAtaA, role: AccountRole.WRITABLE },
            { address: props.depositorAtaB, role: AccountRole.WRITABLE },
            { address: props.payer.address, role: AccountRole.WRITABLE_SIGNER, signer: props.payer },
            { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
        ],
        data: withdrawEncoder.encode({ instruction: 3, amount: props.amount }),
    };
}

export function buildSwap(props: {
    amm: Address;
    pool: Address;
    trader: KeyPairSigner;
    mintA: Address;
    mintB: Address;
    poolAtaA: Address;
    poolAtaB: Address;
    traderAtaA: Address;
    traderAtaB: Address;
    payer: KeyPairSigner;
    swapA: boolean;
    inputAmount: bigint;
    minimumOutputAmount: bigint;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.amm, role: AccountRole.READONLY },
            { address: props.pool, role: AccountRole.READONLY },
            { address: props.trader.address, role: AccountRole.READONLY_SIGNER, signer: props.trader },
            { address: props.mintA, role: AccountRole.READONLY },
            { address: props.mintB, role: AccountRole.READONLY },
            { address: props.poolAtaA, role: AccountRole.WRITABLE },
            { address: props.poolAtaB, role: AccountRole.WRITABLE },
            { address: props.traderAtaA, role: AccountRole.WRITABLE },
            { address: props.traderAtaB, role: AccountRole.WRITABLE },
            { address: props.payer.address, role: AccountRole.WRITABLE_SIGNER, signer: props.payer },
            { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
            { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
        ],
        data: swapEncoder.encode({
            instruction: 4,
            swapA: props.swapA ? 1 : 0,
            inputAmount: props.inputAmount,
            minimumOutputAmount: props.minimumOutputAmount,
        }),
    };
}

export function buildUpdateFee(props: { admin: KeyPairSigner; amm: Address; fee: number; programId: Address }) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.admin.address, role: AccountRole.READONLY_SIGNER, signer: props.admin },
            { address: props.amm, role: AccountRole.WRITABLE },
        ],
        data: updateFeeEncoder.encode({ instruction: 5, fee: props.fee }),
    };
}

export function buildSetPaused(props: { admin: KeyPairSigner; amm: Address; paused: boolean; programId: Address }) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.admin.address, role: AccountRole.READONLY_SIGNER, signer: props.admin },
            { address: props.amm, role: AccountRole.WRITABLE },
        ],
        data: setPausedEncoder.encode({ instruction: 6, paused: props.paused ? 1 : 0 }),
    };
}

export function buildTransferAdmin(props: {
    admin: KeyPairSigner;
    newAdmin: KeyPairSigner;
    amm: Address;
    programId: Address;
}) {
    return {
        programAddress: props.programId,
        accounts: [
            { address: props.admin.address, role: AccountRole.READONLY_SIGNER, signer: props.admin },
            { address: props.newAdmin.address, role: AccountRole.READONLY_SIGNER, signer: props.newAdmin },
            { address: props.amm, role: AccountRole.WRITABLE },
        ],
        data: transferAdminEncoder.encode({ instruction: 7 }),
    };
}

import { getAddressDecoder, getStructDecoder, getU8Decoder, getU16Decoder, getU32Decoder } from '@solana/kit';

export const ammDecoder = getStructDecoder([
    ['creator', getAddressDecoder()],
    ['admin', getAddressDecoder()],
    ['id', getU32Decoder()],
    ['fee', getU16Decoder()],
    ['paused', getU8Decoder()],
    ['bump', getU8Decoder()],
]);

export const poolDecoder = getStructDecoder([
    ['amm', getAddressDecoder()],
    ['mintA', getAddressDecoder()],
    ['mintB', getAddressDecoder()],
    ['bump', getU8Decoder()],
    ['mintLpBump', getU8Decoder()],
]);

# Pinocchio AMM

A constant-product automated market maker for two SPL tokens, implemented with Pinocchio.

## How it works

Each pool stores two token reserves and issues LP tokens to liquidity providers. Swaps use the constant-product formula:

```text
reserve_a * reserve_b = constant
```

The swap fee remains in the pool and accrues to LP token holders.

The first deposit mints:

```text
sqrt(amount_a * amount_b) - minimum_liquidity
```

Later deposits mint LP tokens according to their share of the existing reserves and LP supply.

## Instructions

- `InitializeAmm` creates an AMM configuration.
- `InitializePool` creates a pool, LP mint, and reserve accounts.
- `DepositLiquidity` deposits both pool tokens and mints LP tokens.
- `WithdrawLiquidity` burns LP tokens and returns both pool tokens.
- `SwapExactTokensForTokens` swaps one pool token for the other.
- `UpdateFee` changes the swap fee.
- `SetPaused` pauses deposits, pool creation, and swaps.
- `TransferAdmin` transfers AMM administration.

## Run

```bash
pnpm install
pnpm build-and-test
```

The integration tests use LiteSVM and `@solana/kit`.

## Scope

This is an educational implementation. It does not include deposit or withdrawal slippage limits, oracle pricing, protocol fees, or Token-2022 extensions.

Pool creation and the first deposit are separate instructions. Submit them in one transaction when the initial reserve ratio must not be changed by another transaction.

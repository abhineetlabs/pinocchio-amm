//! Burns LP tokens and withdraws the corresponding pool tokens.
//! The pool configuration signs as the pool's token authority.
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::{
    instructions::{Burn, Transfer},
    state::{Account, Mint},
};

use crate::{ID, MINIMUM_LIQUIDITY, Pool};

pub struct WithdrawLiquidityAccounts<'a> {
    pub pool_config: &'a AccountView,
    pub depositor: &'a AccountView,
    pub mint_lp: &'a mut AccountView,
    pub mint_a: &'a AccountView,
    pub mint_b: &'a AccountView,
    pub pool_ata_a: &'a mut AccountView,
    pub pool_ata_b: &'a mut AccountView,
    pub depositor_ata_lp: &'a mut AccountView,
    pub depositor_ata_a: &'a mut AccountView,
    pub depositor_ata_b: &'a mut AccountView,
    pub payer: &'a mut AccountView,
    pub token_program: &'a AccountView,
    pub associated_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for WithdrawLiquidityAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [
            pool_config,
            depositor,
            mint_lp,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            depositor_ata_lp,
            depositor_ata_a,
            depositor_ata_b,
            payer,
            token_program,
            associated_token_program,
            system_program,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        let (pool_amm, pool_mint_a, pool_mint_b, mint_lp_bump) = {
            let pool = Pool::load_pool(pool_config)?;
            (
                *pool.get_amm(),
                *pool.get_mint_a(),
                *pool.get_mint_b(),
                pool.get_mint_lp_bump(),
            )
        };

        // Validate LP mint
        let expected_mint_lp = Address::derive_address(
            &[
                pool_amm.as_ref(),
                pool_mint_a.as_ref(),
                pool_mint_b.as_ref(),
                b"liquidity",
            ],
            Some(mint_lp_bump[0]),
            &ID,
        );

        if mint_lp.address() != &expected_mint_lp {
            return Err(ProgramError::InvalidSeeds);
        }

        // Validate pool ATAs
        let (expected_pool_ata_a, _) = Address::derive_program_address(
            &[
                pool_config.address().as_ref(),
                pinocchio_token::ID.as_ref(),
                pool_mint_a.as_ref(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        let (expected_pool_ata_b, _) = Address::derive_program_address(
            &[
                pool_config.address().as_ref(),
                pinocchio_token::ID.as_ref(),
                pool_mint_b.as_ref(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if pool_ata_a.address() != &expected_pool_ata_a
            || pool_ata_b.address() != &expected_pool_ata_b
        {
            return Err(ProgramError::InvalidSeeds);
        }

        Ok(Self {
            pool_config,
            depositor,
            mint_lp,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            depositor_ata_lp,
            depositor_ata_a,
            depositor_ata_b,
            payer,
            token_program,
            associated_token_program,
            system_program,
        })
    }
}

pub struct WithdrawLiquidityInstructionData {
    pub amount: u64,
}

impl TryFrom<&[u8]> for WithdrawLiquidityInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 8 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(
            data.try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self { amount })
    }
}

pub struct WithdrawLiquidity<'a> {
    pub accounts: WithdrawLiquidityAccounts<'a>,
    pub instruction_data: WithdrawLiquidityInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for WithdrawLiquidity<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = WithdrawLiquidityAccounts::try_from(accounts)?;
        let instruction_data = WithdrawLiquidityInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> WithdrawLiquidity<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

    pub fn process(&mut self) -> ProgramResult {
        // Load pool balances and LP supply
        let (pool_balance_a, pool_balance_b, mint_lp_supply) = {
            let pool_ata_a = Account::from_account_view(self.accounts.pool_ata_a)?;
            let pool_ata_b = Account::from_account_view(self.accounts.pool_ata_b)?;
            let mint_lp = Mint::from_account_view(self.accounts.mint_lp)?;

            (pool_ata_a.amount(), pool_ata_b.amount(), mint_lp.supply())
        };

        // Calculate pool tokens to withdraw
        let total_liquidity = mint_lp_supply as u128 + MINIMUM_LIQUIDITY as u128;
        let amount_a = ((self.instruction_data.amount as u128) * (pool_balance_a as u128)
            / total_liquidity) as u64;
        let amount_b = ((self.instruction_data.amount as u128) * (pool_balance_b as u128)
            / total_liquidity) as u64;

        // Initialize depositor ATAs
        CreateIdempotent {
            funding_account: self.accounts.payer,
            account: self.accounts.depositor_ata_a,
            wallet: self.accounts.depositor,
            mint: self.accounts.mint_a,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        CreateIdempotent {
            funding_account: self.accounts.payer,
            account: self.accounts.depositor_ata_b,
            wallet: self.accounts.depositor,
            mint: self.accounts.mint_b,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Initialize seeds for signing
        let (pool_amm, pool_mint_a, pool_mint_b, pool_bump) = {
            let pool = Pool::load_pool(self.accounts.pool_config)?;
            (
                *pool.get_amm(),
                *pool.get_mint_a(),
                *pool.get_mint_b(),
                pool.get_bump(),
            )
        };
        let pool_seeds = [
            Seed::from(pool_amm.as_ref()),
            Seed::from(pool_mint_a.as_ref()),
            Seed::from(pool_mint_b.as_ref()),
            Seed::from(&pool_bump),
        ];
        let pool_signers = [Signer::from(&pool_seeds)];

        // Transfer tokens from the pool
        Transfer::new(
            self.accounts.pool_ata_a,
            self.accounts.depositor_ata_a,
            self.accounts.pool_config,
            amount_a,
        )
        .invoke_signed(&pool_signers)?;

        Transfer::new(
            self.accounts.pool_ata_b,
            self.accounts.depositor_ata_b,
            self.accounts.pool_config,
            amount_b,
        )
        .invoke_signed(&pool_signers)?;

        // Burn LP tokens
        Burn::new(
            self.accounts.depositor_ata_lp,
            self.accounts.mint_lp,
            self.accounts.depositor,
            self.instruction_data.amount,
        )
        .invoke()
    }
}

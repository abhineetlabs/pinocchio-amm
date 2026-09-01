//! Deposits a pair of tokens and mints the corresponding LP tokens.
//! The pool configuration signs as the pool's token authority.
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::{
    instructions::{MintTo, Transfer},
    state::Account,
};

use crate::{AmmConfig, ID, Pool};

pub const MINIMUM_LIQUIDITY: u64 = 100;

pub struct DepositLiquidityAccounts<'a> {
    pub amm: &'a AccountView,
    pub pool_config: &'a AccountView,
    pub depositor: &'a AccountView,
    pub mint_lp: &'a mut AccountView,
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

impl<'a> TryFrom<&'a mut [AccountView]> for DepositLiquidityAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [
            amm,
            pool_config,
            depositor,
            mint_lp,
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

        // Validate AMM config
        if amm.address() != &pool_amm {
            return Err(ProgramError::InvalidAccountData);
        }

        let amm_config = AmmConfig::load_amm(amm)?;

        if amm_config.get_paused() != 0 {
            return Err(ProgramError::InvalidArgument);
        }

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
            amm,
            pool_config,
            depositor,
            mint_lp,
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

pub struct DepositLiquidityInstructionData {
    pub amount_a: u64,
    pub amount_b: u64,
}

impl TryFrom<&[u8]> for DepositLiquidityInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 16 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount_a = u64::from_le_bytes(
            data[..8]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let amount_b = u64::from_le_bytes(
            data[8..]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self { amount_a, amount_b })
    }
}

pub struct DepositLiquidity<'a> {
    pub accounts: DepositLiquidityAccounts<'a>,
    pub instruction_data: DepositLiquidityInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for DepositLiquidity<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = DepositLiquidityAccounts::try_from(accounts)?;
        let instruction_data = DepositLiquidityInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> DepositLiquidity<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        // Load token balances
        let (depositor_balance_a, depositor_balance_b, pool_balance_a, pool_balance_b) = {
            let depositor_ata_a = Account::from_account_view(self.accounts.depositor_ata_a)?;
            let depositor_ata_b = Account::from_account_view(self.accounts.depositor_ata_b)?;
            let pool_ata_a = Account::from_account_view(self.accounts.pool_ata_a)?;
            let pool_ata_b = Account::from_account_view(self.accounts.pool_ata_b)?;

            (
                depositor_ata_a.amount(),
                depositor_ata_b.amount(),
                pool_ata_a.amount(),
                pool_ata_b.amount(),
            )
        };

        // Limit deposits to available balances
        let mut amount_a = self.instruction_data.amount_a.min(depositor_balance_a);
        let mut amount_b = self.instruction_data.amount_b.min(depositor_balance_b);
        let pool_creation = pool_balance_a == 0 && pool_balance_b == 0;

        // Preserve the pool ratio
        if !pool_creation {
            let amount_b_required = ((amount_a as u128) * (pool_balance_b as u128))
                .checked_div(pool_balance_a as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            if amount_b_required <= amount_b as u128 {
                amount_b = amount_b_required as u64;
            } else {
                amount_a = ((amount_b as u128) * (pool_balance_a as u128))
                    .checked_div(pool_balance_b as u128)
                    .ok_or(ProgramError::ArithmeticOverflow)? as u64;
            }
        }

        // Calculate LP tokens
        let mut liquidity = ((amount_a as u128) * (amount_b as u128)).isqrt() as u64;

        // Lock minimum liquidity on the first deposit
        if pool_creation {
            liquidity = liquidity
                .checked_sub(MINIMUM_LIQUIDITY)
                .ok_or(ProgramError::InvalidArgument)?;
        }

        // Initialize depositor LP ATA
        CreateIdempotent {
            funding_account: self.accounts.payer,
            account: self.accounts.depositor_ata_lp,
            wallet: self.accounts.depositor,
            mint: self.accounts.mint_lp,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Transfer tokens to the pool
        Transfer::new(
            self.accounts.depositor_ata_a,
            self.accounts.pool_ata_a,
            self.accounts.depositor,
            amount_a,
        )
        .invoke()?;

        Transfer::new(
            self.accounts.depositor_ata_b,
            self.accounts.pool_ata_b,
            self.accounts.depositor,
            amount_b,
        )
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
        let pool_signer = Signer::from(&pool_seeds);

        // Mint LP tokens
        MintTo::new(
            self.accounts.mint_lp,
            self.accounts.depositor_ata_lp,
            self.accounts.pool_config,
            liquidity,
        )
        .invoke_signed(&[pool_signer])
    }
}

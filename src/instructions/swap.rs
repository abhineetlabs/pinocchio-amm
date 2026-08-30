//! Swaps an exact amount of one pool token for the other.
//! The pool configuration signs as the pool's token authority.
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::{instructions::Transfer, state::Account};

use crate::{AmmConfig, Pool};

pub struct SwapExactTokensForTokensAccounts<'a> {
    pub amm: &'a AccountView,
    pub pool_config: &'a AccountView,
    pub trader: &'a AccountView,
    pub mint_a: &'a AccountView,
    pub mint_b: &'a AccountView,
    pub pool_ata_a: &'a mut AccountView,
    pub pool_ata_b: &'a mut AccountView,
    pub trader_ata_a: &'a mut AccountView,
    pub trader_ata_b: &'a mut AccountView,
    pub payer: &'a mut AccountView,
    pub token_program: &'a AccountView,
    pub associated_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for SwapExactTokensForTokensAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [
            amm,
            pool_config,
            trader,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            trader_ata_a,
            trader_ata_b,
            payer,
            token_program,
            associated_token_program,
            system_program,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        let (pool_amm, pool_mint_a, pool_mint_b) = {
            let pool = Pool::load_pool(pool_config)?;
            (*pool.get_amm(), *pool.get_mint_a(), *pool.get_mint_b())
        };

        // Validate AMM config
        if amm.address() != &pool_amm {
            return Err(ProgramError::InvalidAccountData);
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
            trader,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            trader_ata_a,
            trader_ata_b,
            payer,
            token_program,
            associated_token_program,
            system_program,
        })
    }
}

pub struct SwapExactTokensForTokensInstructionData {
    pub swap_a: bool,
    pub input_amount: u64,
    pub min_output_amount: u64,
}

impl TryFrom<&[u8]> for SwapExactTokensForTokensInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 17 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let swap_a = match data[0] {
            0 => false,
            1 => true,
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        let input_amount = u64::from_le_bytes(
            data[1..9]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let min_output_amount = u64::from_le_bytes(
            data[9..]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            swap_a,
            input_amount,
            min_output_amount,
        })
    }
}

pub struct SwapExactTokensForTokens<'a> {
    pub accounts: SwapExactTokensForTokensAccounts<'a>,
    pub instruction_data: SwapExactTokensForTokensInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for SwapExactTokensForTokens<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = SwapExactTokensForTokensAccounts::try_from(accounts)?;
        let instruction_data = SwapExactTokensForTokensInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> SwapExactTokensForTokens<'a> {
    pub const DISCRIMINATOR: &'a u8 = &4;

    pub fn process(&mut self) -> ProgramResult {
        // Initialize trader ATAs
        CreateIdempotent {
            funding_account: self.accounts.payer,
            account: self.accounts.trader_ata_a,
            wallet: self.accounts.trader,
            mint: self.accounts.mint_a,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        CreateIdempotent {
            funding_account: self.accounts.payer,
            account: self.accounts.trader_ata_b,
            wallet: self.accounts.trader,
            mint: self.accounts.mint_b,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Load token balances
        let (trader_balance_a, trader_balance_b, pool_balance_a, pool_balance_b) = {
            let trader_ata_a = Account::from_account_view(self.accounts.trader_ata_a)?;
            let trader_ata_b = Account::from_account_view(self.accounts.trader_ata_b)?;
            let pool_ata_a = Account::from_account_view(self.accounts.pool_ata_a)?;
            let pool_ata_b = Account::from_account_view(self.accounts.pool_ata_b)?;

            (
                trader_ata_a.amount(),
                trader_ata_b.amount(),
                pool_ata_a.amount(),
                pool_ata_b.amount(),
            )
        };

        // Limit input to available balance
        let input = if self.instruction_data.swap_a {
            self.instruction_data.input_amount.min(trader_balance_a)
        } else {
            self.instruction_data.input_amount.min(trader_balance_b)
        };

        // Apply trading fee
        let fee = AmmConfig::load_amm(self.accounts.amm)?.get_fee();
        let fee_amount = ((input as u128) * (fee as u128) / 10_000) as u64;
        let taxed_input = input - fee_amount;

        // Calculate output tokens
        let output = if self.instruction_data.swap_a {
            ((taxed_input as u128) * (pool_balance_b as u128))
                .checked_div(pool_balance_a as u128 + taxed_input as u128)
                .ok_or(ProgramError::ArithmeticOverflow)? as u64
        } else {
            ((taxed_input as u128) * (pool_balance_a as u128))
                .checked_div(pool_balance_b as u128 + taxed_input as u128)
                .ok_or(ProgramError::ArithmeticOverflow)? as u64
        };

        if output < self.instruction_data.min_output_amount {
            return Err(ProgramError::InvalidArgument);
        }

        // Store invariant before swap
        let invariant = (pool_balance_a as u128) * (pool_balance_b as u128);

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

        // Transfer tokens
        if self.instruction_data.swap_a {
            Transfer::new(
                self.accounts.trader_ata_a,
                self.accounts.pool_ata_a,
                self.accounts.trader,
                input,
            )
            .invoke()?;

            Transfer::new(
                self.accounts.pool_ata_b,
                self.accounts.trader_ata_b,
                self.accounts.pool_config,
                output,
            )
            .invoke_signed(&[pool_signer])?;
        } else {
            Transfer::new(
                self.accounts.pool_ata_a,
                self.accounts.trader_ata_a,
                self.accounts.pool_config,
                output,
            )
            .invoke_signed(&[pool_signer])?;

            Transfer::new(
                self.accounts.trader_ata_b,
                self.accounts.pool_ata_b,
                self.accounts.trader,
                input,
            )
            .invoke()?;
        }

        // Validate invariant after swap
        let new_invariant = {
            let pool_ata_a = Account::from_account_view(self.accounts.pool_ata_a)?;
            let pool_ata_b = Account::from_account_view(self.accounts.pool_ata_b)?;

            (pool_ata_a.amount() as u128) * (pool_ata_b.amount() as u128)
        };

        if invariant > new_invariant {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

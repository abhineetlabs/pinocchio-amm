//! Initializes the AMM configuration account.
//! The account stores the creator, admin, ID, fee, pause state, and canonical bump.
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_system::instructions::CreateAccount;

use crate::{AmmConfig, ID};

pub struct InitializeAmmAccounts<'a> {
    pub payer: &'a mut AccountView,
    pub admin: &'a AccountView,
    pub amm: &'a mut AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for InitializeAmmAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [payer, admin, amm, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Basic Validation
        if system_program.address() != &pinocchio_system::ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            payer,
            admin,
            amm,
            system_program,
        })
    }
}

pub struct InitializeAmmInstructionData {
    pub id: u32,
    pub fee: u16,
}

impl<'a> TryFrom<&'a [u8]> for InitializeAmmInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != 4 + 2 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let id = u32::from_le_bytes(
            data[0..4]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        let fee = u16::from_le_bytes(
            data[4..6]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        // Reject if fee greater than 10_000 basis points
        if fee >= 10_000 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { id, fee })
    }
}

pub struct InitializeAmm<'a> {
    accounts: InitializeAmmAccounts<'a>,
    instruction_data: InitializeAmmInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &'a [u8])> for InitializeAmm<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = InitializeAmmAccounts::try_from(accounts)?;
        let instruction_data = InitializeAmmInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> InitializeAmm<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        let amm_id = self.instruction_data.id.to_le_bytes();

        // Validate canonical PDA
        let (expected_amm, canonical_bump) = Address::derive_program_address(
            &[b"amm", self.accounts.admin.address().as_array(), &amm_id],
            &ID,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;

        let canonical_bump = [canonical_bump];

        if self.accounts.amm.address() != &expected_amm {
            return Err(ProgramError::InvalidAccountData);
        }

        // Initialize seeds for signing
        let seeds = &[
            Seed::from(b"amm"),
            Seed::from(self.accounts.admin.address().as_array()),
            Seed::from(&amm_id),
            Seed::from(&canonical_bump),
        ];

        let signer = Signer::from(seeds);

        CreateAccount::with_minimum_balance(
            self.accounts.payer,
            self.accounts.amm,
            AmmConfig::LEN as u64,
            &ID,
            None,
        )?
        .invoke_signed(&[signer])?;

        // Populate config
        let mut amm_config = AmmConfig::load_amm_mut(self.accounts.amm)?;

        amm_config.set_all(
            *self.accounts.admin.address(),
            *self.accounts.admin.address(),
            self.instruction_data.id,
            self.instruction_data.fee,
            0,
            canonical_bump,
        );

        Ok(())
    }
}

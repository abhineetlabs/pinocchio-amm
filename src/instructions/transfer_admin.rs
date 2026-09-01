//! Transfers AMM administration to a new authority.
//! Both the current and new admins must sign.
use pinocchio::{AccountView, ProgramResult, error::ProgramError};

use crate::AmmConfig;

pub struct TransferAdminAccounts<'a> {
    pub admin: &'a AccountView,
    pub new_admin: &'a AccountView,
    pub amm: &'a mut AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for TransferAdminAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [admin, new_admin, amm] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() || !new_admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !amm.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        {
            let amm_config = AmmConfig::load_amm(amm)?;

            if admin.address() != amm_config.get_admin() {
                return Err(ProgramError::IncorrectAuthority);
            }
        }

        Ok(Self {
            admin,
            new_admin,
            amm,
        })
    }
}

pub struct TransferAdminInstructionData;

impl TryFrom<&[u8]> for TransferAdminInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if !data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self)
    }
}

pub struct TransferAdmin<'a> {
    pub accounts: TransferAdminAccounts<'a>,
    pub instruction_data: TransferAdminInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for TransferAdmin<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = TransferAdminAccounts::try_from(accounts)?;
        let instruction_data = TransferAdminInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> TransferAdmin<'a> {
    pub const DISCRIMINATOR: &'a u8 = &7;

    pub fn process(&mut self) -> ProgramResult {
        let mut amm_config = AmmConfig::load_amm_mut(self.accounts.amm)?;
        amm_config.set_admin(*self.accounts.new_admin.address());

        Ok(())
    }
}

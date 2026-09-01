//! Updates the AMM trading fee.
//! Only the current admin can update the fee.
use pinocchio::{AccountView, ProgramResult, error::ProgramError};

use crate::AmmConfig;

pub struct UpdateFeeAccounts<'a> {
    pub admin: &'a AccountView,
    pub amm: &'a mut AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for UpdateFeeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [admin, amm] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
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

        Ok(Self { admin, amm })
    }
}

pub struct UpdateFeeInstructionData {
    pub fee: u16,
}

impl TryFrom<&[u8]> for UpdateFeeInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 2 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let fee = u16::from_le_bytes(
            data.try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        if fee >= 10_000 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { fee })
    }
}

pub struct UpdateFee<'a> {
    pub accounts: UpdateFeeAccounts<'a>,
    pub instruction_data: UpdateFeeInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for UpdateFee<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = UpdateFeeAccounts::try_from(accounts)?;
        let instruction_data = UpdateFeeInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> UpdateFee<'a> {
    pub const DISCRIMINATOR: &'a u8 = &5;

    pub fn process(&mut self) -> ProgramResult {
        let mut amm_config = AmmConfig::load_amm_mut(self.accounts.amm)?;
        amm_config.set_fee(self.instruction_data.fee);

        Ok(())
    }
}

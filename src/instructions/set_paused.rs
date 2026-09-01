//! Updates the AMM pause state.
//! Only the current admin can pause or resume the AMM.
use pinocchio::{AccountView, ProgramResult, error::ProgramError};

use crate::AmmConfig;

pub struct SetPausedAccounts<'a> {
    pub admin: &'a AccountView,
    pub amm: &'a mut AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for SetPausedAccounts<'a> {
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

pub struct SetPausedInstructionData {
    pub paused: u8,
}

impl TryFrom<&[u8]> for SetPausedInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != 1 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let paused = match data[0] {
            0 => 0,
            1 => 1,
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        Ok(Self { paused })
    }
}

pub struct SetPaused<'a> {
    pub accounts: SetPausedAccounts<'a>,
    pub instruction_data: SetPausedInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for SetPaused<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = SetPausedAccounts::try_from(accounts)?;
        let instruction_data = SetPausedInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> SetPaused<'a> {
    pub const DISCRIMINATOR: &'a u8 = &6;

    pub fn process(&mut self) -> ProgramResult {
        let mut amm_config = AmmConfig::load_amm_mut(self.accounts.amm)?;
        amm_config.set_paused(self.instruction_data.paused);

        Ok(())
    }
}

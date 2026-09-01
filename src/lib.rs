#![no_std]
use pinocchio::{
    AccountView, Address, ProgramResult, entrypoint, error::ProgramError, nostd_panic_handler,
};

entrypoint!(process_instruction);
nostd_panic_handler!();

pub mod instructions;
pub mod state;
pub use instructions::*;
pub use state::*;

pub const ID: Address = Address::from_str_const("GGZzCxQb9D7v84Ai1WkQgeqRx79j8pRZfk8yQmF3Jvqo");

pub fn process_instruction(
    _program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    match data.split_first() {
        Some((InitializeAmm::DISCRIMINATOR, data)) => {
            InitializeAmm::try_from((accounts, data))?.process()
        }
        Some((InitializePool::DISCRIMINATOR, data)) => {
            InitializePool::try_from((accounts, data))?.process()
        }
        Some((DepositLiquidity::DISCRIMINATOR, data)) => {
            DepositLiquidity::try_from((accounts, data))?.process()
        }
        Some((WithdrawLiquidity::DISCRIMINATOR, data)) => {
            WithdrawLiquidity::try_from((accounts, data))?.process()
        }
        Some((SwapExactTokensForTokens::DISCRIMINATOR, data)) => {
            SwapExactTokensForTokens::try_from((accounts, data))?.process()
        }
        Some((UpdateFee::DISCRIMINATOR, data)) => UpdateFee::try_from((accounts, data))?.process(),
        Some((SetPaused::DISCRIMINATOR, data)) => SetPaused::try_from((accounts, data))?.process(),
        Some((TransferAdmin::DISCRIMINATOR, data)) => {
            TransferAdmin::try_from((accounts, data))?.process()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

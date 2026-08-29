/// This instruction initializes a new LP Pair
use pinocchio::{
    AccountView, Address, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccountAllowPrefund;
use pinocchio_token::{instructions::InitializeMint2, state::Mint};

use crate::{AmmConfig, ID, Pool};

pub struct InitializePoolAccounts<'a> {
    pub payer: &'a mut AccountView,
    pub amm: &'a AccountView,
    pub pool_config: &'a mut AccountView,
    pub mint_lp: &'a mut AccountView,
    pub mint_a: &'a AccountView,
    pub mint_b: &'a AccountView,
    pub pool_ata_a: &'a mut AccountView,
    pub pool_ata_b: &'a mut AccountView,
    pub token_program: &'a AccountView,
    pub associated_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a mut [AccountView]> for InitializePoolAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let [
            payer,
            amm,
            pool_config,
            mint_lp,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            token_program,
            associated_token_program,
            system_program,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Basic checks
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !payer.is_writable()
            || !pool_config.is_writable()
            || !mint_lp.is_writable()
            || !pool_ata_a.is_writable()
            || !pool_ata_b.is_writable()
        {
            return Err(ProgramError::InvalidAccountData);
        }

        if token_program.address() != &pinocchio_token::ID
            || associated_token_program.address() != &pinocchio_associated_token_account::ID
            || system_program.address() != &pinocchio_system::ID
        {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Validate AMM config
        let amm_config = AmmConfig::load_amm(amm)?;
        let amm_id = amm_config.get_id().to_le_bytes();
        let expected_amm = Address::derive_address(
            &[b"amm", amm_config.get_admin().as_ref(), &amm_id],
            Some(amm_config.get_bump()[0]),
            &ID,
        );

        if amm.address() != &expected_amm {
            return Err(ProgramError::InvalidSeeds);
        }

        // Validate mints
        let mint_a_data = Mint::from_account_view(mint_a)?;
        let mint_b_data = Mint::from_account_view(mint_b)?;

        if !mint_a_data.is_initialized() || !mint_b_data.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        if mint_a.address().as_ref() >= mint_b.address().as_ref() {
            return Err(ProgramError::InvalidArgument);
        }

        let (expected_pool_ata_a, _) = Address::derive_program_address(
            &[
                pool_config.address().as_ref(),
                pinocchio_token::ID.as_ref(),
                mint_a.address().as_ref(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        let (expected_pool_ata_b, _) = Address::derive_program_address(
            &[
                pool_config.address().as_ref(),
                pinocchio_token::ID.as_ref(),
                mint_b.address().as_ref(),
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
            payer,
            amm,
            pool_config,
            mint_lp,
            mint_a,
            mint_b,
            pool_ata_a,
            pool_ata_b,
            token_program,
            associated_token_program,
            system_program,
        })
    }
}

pub struct InitializePoolInstructionData;

impl TryFrom<&[u8]> for InitializePoolInstructionData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if !data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self)
    }
}

pub struct InitializePool<'a> {
    pub accounts: InitializePoolAccounts<'a>,
    pub instruction_data: InitializePoolInstructionData,
}

impl<'a> TryFrom<(&'a mut [AccountView], &[u8])> for InitializePool<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a mut [AccountView], &[u8])) -> Result<Self, Self::Error> {
        let accounts = InitializePoolAccounts::try_from(accounts)?;
        let instruction_data = InitializePoolInstructionData::try_from(data)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}

impl<'a> InitializePool<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        // Derive pool config PDA
        let (expected_pool_config, pool_bump) = Address::derive_program_address(
            &[
                self.accounts.amm.address().as_ref(),
                self.accounts.mint_a.address().as_ref(),
                self.accounts.mint_b.address().as_ref(),
            ],
            &ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if self.accounts.pool_config.address() != &expected_pool_config {
            return Err(ProgramError::InvalidSeeds);
        }

        let pool_bump = [pool_bump];
        let pool_seeds = [
            Seed::from(self.accounts.amm.address().as_ref()),
            Seed::from(self.accounts.mint_a.address().as_ref()),
            Seed::from(self.accounts.mint_b.address().as_ref()),
            Seed::from(&pool_bump),
        ];
        let pool_signer = Signer::from(&pool_seeds);

        // Derive LP mint PDA
        let (expected_mint_lp, mint_lp_bump) = Address::derive_program_address(
            &[
                self.accounts.amm.address().as_ref(),
                self.accounts.mint_a.address().as_ref(),
                self.accounts.mint_b.address().as_ref(),
                b"liquidity",
            ],
            &ID,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if self.accounts.mint_lp.address() != &expected_mint_lp {
            return Err(ProgramError::InvalidSeeds);
        }

        let mint_lp_bump = [mint_lp_bump];
        let mint_lp_seeds = [
            Seed::from(self.accounts.amm.address().as_ref()),
            Seed::from(self.accounts.mint_a.address().as_ref()),
            Seed::from(self.accounts.mint_b.address().as_ref()),
            Seed::from(b"liquidity"),
            Seed::from(&mint_lp_bump),
        ];
        let mint_lp_signer = Signer::from(&mint_lp_seeds);

        // Initialize pool config
        CreateAccountAllowPrefund::with_minimum_balance(
            self.accounts.payer,
            self.accounts.pool_config,
            Pool::LEN as u64,
            &ID,
            None,
        )?
        .invoke_signed(&[pool_signer])?;

        // Initialize LP mint
        CreateAccountAllowPrefund::with_minimum_balance(
            self.accounts.payer,
            self.accounts.mint_lp,
            Mint::LEN as u64,
            &pinocchio_token::ID,
            None,
        )?
        .invoke_signed(&[mint_lp_signer])?;

        InitializeMint2::new(
            self.accounts.mint_lp,
            6,
            self.accounts.pool_config.address(),
            None,
        )
        .invoke()?;

        // Initialize pool ATAs
        Create {
            funding_account: self.accounts.payer,
            account: self.accounts.pool_ata_a,
            wallet: self.accounts.pool_config,
            mint: self.accounts.mint_a,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        Create {
            funding_account: self.accounts.payer,
            account: self.accounts.pool_ata_b,
            wallet: self.accounts.pool_config,
            mint: self.accounts.mint_b,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Populate pool config
        let mut pool_config = Pool::load_pool_mut(self.accounts.pool_config)?;
        pool_config.set_all(
            *self.accounts.amm.address(),
            *self.accounts.mint_a.address(),
            *self.accounts.mint_b.address(),
            pool_bump,
            mint_lp_bump,
        );

        Ok(())
    }
}

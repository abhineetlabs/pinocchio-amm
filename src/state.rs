use pinocchio::{
    AccountView, Address,
    account::{Ref, RefMut},
    error::ProgramError,
};

use crate::ID;

// Config for AMM
#[repr(C, packed)]
pub struct AmmConfig {
    // Creator used for PDA derivation
    creator: Address,
    // Admin authority
    admin: Address,
    // Random 4 byte value
    id: u32,
    // Fee in basis points
    fee: u16,
    // Paused state
    paused: u8,
    bump: [u8; 1],
}

impl AmmConfig {
    pub const LEN: usize = size_of::<Self>();

    // Pointer casting functions
    #[inline(always)]
    unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const AmmConfig) }
    }
    #[inline(always)]
    unsafe fn from_bytes_unchecked_mut(bytes: &mut [u8]) -> &mut Self {
        unsafe { &mut *(bytes.as_mut_ptr() as *mut AmmConfig) }
    }
}

// Reader functions
impl AmmConfig {
    #[inline(always)]
    pub fn load_amm(account: &AccountView) -> Result<Ref<'_, Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        if account.owner() != &ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Ref::map(account.try_borrow()?, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }

    // Getter functions for safe field access
    #[inline(always)]
    pub fn get_creator(&self) -> &Address {
        &self.creator
    }

    #[inline(always)]
    pub fn get_admin(&self) -> &Address {
        &self.admin
    }

    #[inline(always)]
    pub fn get_id(&self) -> u32 {
        self.id
    }

    #[inline(always)]
    pub fn get_fee(&self) -> u16 {
        self.fee
    }

    #[inline(always)]
    pub fn get_paused(&self) -> u8 {
        self.paused
    }

    #[inline(always)]
    pub fn get_bump(&self) -> [u8; 1] {
        self.bump
    }
}

// Writer functions
impl AmmConfig {
    #[inline(always)]
    pub fn load_amm_mut(account: &mut AccountView) -> Result<RefMut<'_, Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        if account.owner() != &ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(RefMut::map(account.try_borrow_mut()?, |data| unsafe {
            Self::from_bytes_unchecked_mut(data)
        }))
    }

    // Setter functions
    #[inline(always)]
    pub fn set_admin(&mut self, admin: Address) {
        self.admin = admin;
    }

    #[inline(always)]
    pub fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    #[inline(always)]
    pub fn set_fee(&mut self, fee: u16) {
        self.fee = fee;
    }

    #[inline(always)]
    pub fn set_paused(&mut self, paused: u8) {
        self.paused = paused;
    }

    #[inline(always)]
    pub fn set_bump(&mut self, bump: [u8; 1]) {
        self.bump = bump;
    }

    #[inline(always)]
    pub fn set_all(
        &mut self,
        creator: Address,
        admin: Address,
        id: u32,
        fee: u16,
        paused: u8,
        bump: [u8; 1],
    ) {
        self.creator = creator;
        self.admin = admin;
        self.id = id;
        self.fee = fee;
        self.paused = paused;
        self.bump = bump
    }
}

// Config for Pool
#[repr(C, packed)]
pub struct Pool {
    // AMM Config
    amm: Address,
    // Mint A
    mint_a: Address,
    // Mint B
    mint_b: Address,
    // Pool Config bump
    bump: [u8; 1],
    // LP Mint bump
    mint_lp_bump: [u8; 1],
}

impl Pool {
    pub const LEN: usize = size_of::<Self>();

    // Pointer casting functions
    #[inline(always)]
    unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes.as_ptr() as *const Pool) }
    }
    #[inline(always)]
    unsafe fn from_bytes_unchecked_mut(bytes: &mut [u8]) -> &mut Self {
        unsafe { &mut *(bytes.as_mut_ptr() as *mut Pool) }
    }
}

// Reader functions
impl Pool {
    #[inline(always)]
    pub fn load_pool(account: &AccountView) -> Result<Ref<'_, Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        if account.owner() != &ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Ref::map(account.try_borrow()?, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }

    // Getter functions for safe field access
    #[inline(always)]
    pub fn get_amm(&self) -> &Address {
        &self.amm
    }

    #[inline(always)]
    pub fn get_mint_a(&self) -> &Address {
        &self.mint_a
    }

    #[inline(always)]
    pub fn get_mint_b(&self) -> &Address {
        &self.mint_b
    }

    #[inline(always)]
    pub fn get_bump(&self) -> [u8; 1] {
        self.bump
    }

    #[inline(always)]
    pub fn get_mint_lp_bump(&self) -> [u8; 1] {
        self.mint_lp_bump
    }
}

// Writer functions
impl Pool {
    #[inline(always)]
    pub fn load_pool_mut(account: &mut AccountView) -> Result<RefMut<'_, Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        if account.owner() != &ID {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(RefMut::map(account.try_borrow_mut()?, |data| unsafe {
            Self::from_bytes_unchecked_mut(data)
        }))
    }

    // Setter functions
    #[inline(always)]
    pub fn set_amm(&mut self, amm: Address) {
        self.amm = amm;
    }

    #[inline(always)]
    pub fn set_mint_a(&mut self, mint_a: Address) {
        self.mint_a = mint_a;
    }

    #[inline(always)]
    pub fn set_mint_b(&mut self, mint_b: Address) {
        self.mint_b = mint_b;
    }

    #[inline(always)]
    pub fn set_bump(&mut self, bump: [u8; 1]) {
        self.bump = bump;
    }

    #[inline(always)]
    pub fn set_mint_lp_bump(&mut self, mint_lp_bump: [u8; 1]) {
        self.mint_lp_bump = mint_lp_bump;
    }

    #[inline(always)]
    pub fn set_all(
        &mut self,
        amm: Address,
        mint_a: Address,
        mint_b: Address,
        bump: [u8; 1],
        mint_lp_bump: [u8; 1],
    ) {
        self.amm = amm;
        self.mint_a = mint_a;
        self.mint_b = mint_b;
        self.bump = bump;
        self.mint_lp_bump = mint_lp_bump
    }
}

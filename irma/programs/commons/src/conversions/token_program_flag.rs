use crate::*;
use crate::dlmm::types::TokenProgramFlags;
use std::ops::Deref;
use std::result::Result;
use std::result::Result::Ok;

pub struct TokenProgramFlagWrapper(TokenProgramFlags);

impl Deref for TokenProgramFlagWrapper {
    type Target = TokenProgramFlags;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<u8> for TokenProgramFlagWrapper {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TokenProgramFlagWrapper(TokenProgramFlags::TokenProgram)),
            1 => Ok(TokenProgramFlagWrapper(TokenProgramFlags::TokenProgram2022)),
            _ => Err(Error::msg("Invalid TokenProgramFlags value"))?,
        }
    }
}

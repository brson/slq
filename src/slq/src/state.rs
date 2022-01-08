#![allow(unused)]

use anyhow::{Result, anyhow};
use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info, next_account_infos},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    system_instruction,
    program::invoke_signed,
    msg,
};
use solana_program::instruction::{AccountMeta, Instruction};
use std::convert::{TryFrom, TryInto};

pub struct SlqInstance {
    pub admin_config: AdminConfig,
}

pub const MAX_ADMIN_ACCOUNTS: usize = 16;

pub struct AdminConfig {
    pub approval_threshold: u8,
    pub admin_accounts: [Pubkey; MAX_ADMIN_ACCOUNTS],
}

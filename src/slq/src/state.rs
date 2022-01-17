#![allow(unused)]

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use std::convert::{TryFrom, TryInto};

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct SlqInstance {
    pub admin_config: AdminConfig,
}

pub const MAX_ADMIN_ACCOUNTS: usize = 6;

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct AdminConfig {
    pub approval_threshold: u8,
    pub admin_accounts: [Pubkey; MAX_ADMIN_ACCOUNTS],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct MultisigConfigInstance {
    pub approval_threshold: u8,
    pub owners: [Pubkey; crate::multisig::MAX_MULTISIG_OWNERS],
}

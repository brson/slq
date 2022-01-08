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

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqAdminInstruction,
) -> ProgramResult {
    todo!()
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqAdminInstruction {
    Init(Init),
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Init {
    pub instance: String,
    pub approval_threshold: u8,
    pub admin_accounts: Vec<Pubkey>,
}

impl Init {
    pub fn build_instruction(
        program_id: &Pubkey,
        approval_threshold: u8,
        admin_accounts: &[Pubkey]
    ) -> Result<Instruction> {
        todo!()
    }
}

#![allow(unused)]

use anyhow::{anyhow, bail, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::get_instance_packed_len;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
};
use std::convert::{TryFrom, TryInto};

use crate::state::AdminConfig;
use crate::state::SlqInstance;
use crate::state::MAX_ADMIN_ACCOUNTS;
use crate::SlqInstruction;

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqAdminInstruction,
) -> ProgramResult {
    match instr {
        SlqAdminInstruction::ChangeApprovalThreshold(instr) => todo!(),
        SlqAdminInstruction::AddAdminAccount(instr) => todo!(),
        SlqAdminInstruction::RemoveAdminAccount(instr) => todo!(),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqAdminInstruction {
    ChangeApprovalThreshold(ChangeApprovalThresholdAdmin),
    AddAdminAccount(AddAdminAccountAdmin),
    RemoveAdminAccount(RemoveAdminAccountAdmin),
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ChangeApprovalThresholdAdmin {

}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AddAdminAccountAdmin {

}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RemoveAdminAccountAdmin {

}

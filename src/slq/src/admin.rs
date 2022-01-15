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
use crate::init::make_instance_pda;

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

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: instance_pda - pda, writable, uninitialized
/// - 2: system_program - executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ChangeApprovalThresholdAdmin {
    instance_name: String,
    approval_threshold: u8,
}

impl ChangeApprovalThresholdAdmin {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        instance_name: String,
        approval_threshold: u8,
    ) -> Result<Instruction> {

        // todo varification

        let (instance_pda, _) = make_instance_pda(program_id, &instance_name);

        let instr =
            SlqInstruction::Admin(SlqAdminInstruction::ChangeApprovalThreshold(ChangeApprovalThresholdAdmin {
                instance_name,
                approval_threshold,
            }));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(instance_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AddAdminAccountAdmin {

}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RemoveAdminAccountAdmin {

}

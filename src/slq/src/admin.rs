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

use crate::init::{create_admin_accounts_array, make_instance_pda, verify_pda};
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
        SlqAdminInstruction::ChangeApprovalThreshold(instr) => instr.exec(program_id, accounts),
        SlqAdminInstruction::AddAdminAccount(instr) => instr.exec(program_id, accounts),
        SlqAdminInstruction::RemoveAdminAccount(instr) => instr.exec(program_id, accounts),
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
/// - 1: instance_pda - pda, writable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ChangeApprovalThresholdAdmin {
    instance_name: String,
    approval_threshold: u8,
    instance_pda_bump_seed: u8,
}

impl ChangeApprovalThresholdAdmin {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        instance_name: String,
        approval_threshold: u8,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr = SlqInstruction::Admin(SlqAdminInstruction::ChangeApprovalThreshold(
            ChangeApprovalThresholdAdmin {
                instance_name,
                approval_threshold,
                instance_pda_bump_seed,
            },
        ));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(instance_pda, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let rent_payer = next_account_info(accounts_iter)?;
        let instance_pda = next_account_info(accounts_iter)?;

        {
            assert!(rent_payer.is_writable);
            assert!(rent_payer.is_signer);
            assert!(instance_pda.is_writable);
            assert_eq!(instance_pda.owner, program_id, "unexpected program id");

            verify_pda(
                program_id,
                &self.instance_name,
                instance_pda.key,
                self.instance_pda_bump_seed,
                make_instance_pda,
            );
        }

        let mut instance = SlqInstance::try_from_slice(&instance_pda.data.borrow_mut())?;
        instance.admin_config.approval_threshold = self.approval_threshold;
        instance.serialize(&mut *instance_pda.data.borrow_mut())?;

        Ok(())
    }
}

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: instance_pda - pda, writable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AddAdminAccountAdmin {
    instance_name: String,
    new_admin_account: Pubkey,
    instance_pda_bump_seed: u8,
}

impl AddAdminAccountAdmin {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        instance_name: String,
        new_admin_account: Pubkey,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr =
            SlqInstruction::Admin(SlqAdminInstruction::AddAdminAccount(AddAdminAccountAdmin {
                instance_name,
                new_admin_account,
                instance_pda_bump_seed,
            }));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(instance_pda, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let rent_payer = next_account_info(accounts_iter)?;
        let instance_pda = next_account_info(accounts_iter)?;

        {
            assert!(rent_payer.is_writable);
            assert!(rent_payer.is_signer);
            assert!(instance_pda.is_writable);
            assert_eq!(instance_pda.owner, program_id, "unexpected program id");

            verify_pda(
                program_id,
                &self.instance_name,
                instance_pda.key,
                self.instance_pda_bump_seed,
                make_instance_pda,
            );
        }

        let mut instance = SlqInstance::try_from_slice(&instance_pda.data.borrow_mut())?;

        let mut admin_accounts = instance
            .admin_config
            .admin_accounts
            .iter()
            .filter(|account| **account != Pubkey::default())
            .copied()
            .collect::<Vec<Pubkey>>();

        admin_accounts.push(self.new_admin_account);

        instance.admin_config.admin_accounts = create_admin_accounts_array(&admin_accounts);

        instance.serialize(&mut *instance_pda.data.borrow_mut())?;

        Ok(())
    }
}

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: instance_pda - pda, writable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RemoveAdminAccountAdmin {
    instance_name: String,
    to_remove_admin_account: Pubkey,
    instance_pda_bump_seed: u8,
}

impl RemoveAdminAccountAdmin {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        instance_name: String,
        to_remove_admin_account: Pubkey,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr =
            SlqInstruction::Admin(SlqAdminInstruction::RemoveAdminAccount(RemoveAdminAccountAdmin {
                instance_name,
                to_remove_admin_account,
                instance_pda_bump_seed,
            }));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(instance_pda, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let rent_payer = next_account_info(accounts_iter)?;
        let instance_pda = next_account_info(accounts_iter)?;

        {
            assert!(rent_payer.is_writable);
            assert!(rent_payer.is_signer);
            assert!(instance_pda.is_writable);
            assert_eq!(instance_pda.owner, program_id, "unexpected program id");

            verify_pda(
                program_id,
                &self.instance_name,
                instance_pda.key,
                self.instance_pda_bump_seed,
                make_instance_pda,
            );
        }

        let mut instance = SlqInstance::try_from_slice(&instance_pda.data.borrow_mut())?;

        let mut admin_accounts = instance
            .admin_config
            .admin_accounts
            .iter()
            .filter(|account| **account != Pubkey::default() && **account != self.to_remove_admin_account)
            .copied()
            .collect::<Vec<Pubkey>>();

        instance.admin_config.admin_accounts = create_admin_accounts_array(&admin_accounts);

        instance.serialize(&mut *instance_pda.data.borrow_mut())?;

        Ok(())
    }
}

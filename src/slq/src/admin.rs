#![allow(unused)]

use anyhow::{Result, anyhow, bail};
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

use crate::SlqInstruction;
use crate::state::MAX_ADMIN_ACCOUNTS;

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqAdminInstruction,
) -> ProgramResult {
    match instr {
        SlqAdminInstruction::Init(instr) => {
            todo!()
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqAdminInstruction {
    Init(Init),
}

/// # Accounts
///
/// - 0: instance_pda: pda, writable, owner=program, uninitialized
/// - 1: system_Program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Init {
    pub instance_name: String,
    pub approval_threshold: u8,
    pub admin_accounts: Vec<Pubkey>,
    pub instance_pda_bump_seed: u8,
}

impl Init {
    pub fn build_instruction(
        program_id: &Pubkey,
        instance_name: String,
        approval_threshold: u8,
        admin_accounts: Vec<Pubkey>
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = instance_pda(program_id, &instance_name);

        let instr = Init {
            instance_name,
            approval_threshold,
            admin_accounts,
            instance_pda_bump_seed,
        };

        instr.validate()?;

        let instr = SlqInstruction::Admin(
            SlqAdminInstruction::Init(instr)
        );

        let accounts = vec![
            AccountMeta::new(instance_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(
            *program_id,
            &instr,
            accounts,
        ))
    }

    fn validate(&self) -> Result<()> {
        if self.approval_threshold == 0 {
            bail!("approval threshold must be greater than 0");
        }

        if self.admin_accounts.len() == 0 {
            bail!("must have greater than 0 admin accounts");
        }

        if self.admin_accounts.len() > MAX_ADMIN_ACCOUNTS {
            bail!("number of admin accounts must be not be greater than {}", MAX_ADMIN_ACCOUNTS);
        }

        if usize::from(self.approval_threshold) > self.admin_accounts.len() {
            bail!("approval threshold must not be greater than number of admin accounts");
        }

        let mut sorted_accounts = self.admin_accounts.clone();
        sorted_accounts.sort_unstable();
        sorted_accounts.dedup();

        if sorted_accounts.len() != self.admin_accounts.len() {
            bail!("must not have duplicate admin accounts");
        }

        Ok(())
    }

    fn exec(&self, program_id: &Pubkey) -> ProgramResult {
        todo!()
    }
}

fn instance_pda(
    program_id: &Pubkey,
    instance_name: &str,
) -> (Pubkey, u8) {
    let seeds = &[b"instance", instance_name.as_bytes()];
    Pubkey::find_program_address(seeds, program_id)
}
                

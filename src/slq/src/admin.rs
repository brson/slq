#![allow(unused)]

use anyhow::{anyhow, bail, Result};
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

use crate::state::MAX_ADMIN_ACCOUNTS;
use crate::SlqInstruction;

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqAdminInstruction,
) -> ProgramResult {
    match instr {
        SlqAdminInstruction::Init(instr) => instr.exec(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqAdminInstruction {
    Init(Init),
}

/// # Accounts
///
/// - 0: instance_pda: pda, writable, owner=system_program, uninitialized
/// - 1: system_program: executable
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
        admin_accounts: Vec<Pubkey>,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr = Init {
            instance_name,
            approval_threshold,
            admin_accounts,
            instance_pda_bump_seed,
        };

        instr.validate()?;

        let instr = SlqInstruction::Admin(SlqAdminInstruction::Init(instr));

        let accounts = vec![
            AccountMeta::new(instance_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn validate(&self) -> Result<()> {
        if self.approval_threshold == 0 {
            bail!("approval threshold must be greater than 0");
        }

        if self.admin_accounts.len() == 0 {
            bail!("must have greater than 0 admin accounts");
        }

        if self.admin_accounts.len() > MAX_ADMIN_ACCOUNTS {
            bail!(
                "number of admin accounts must be not be greater than {}",
                MAX_ADMIN_ACCOUNTS
            );
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

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let instance_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        assert!(instance_pda.is_writable);
        assert_eq!(instance_pda.owner, system_program.key);
        assert_eq!(system_program.key, &system_program::ID);
        assert!(system_program.executable);

        verify_pda(
            program_id,
            &self.instance_name,
            instance_pda.key,
            self.instance_pda_bump_seed,
            make_instance_pda,
        );

        todo!()
    }
}

fn make_instance_pda(program_id: &Pubkey, instance_name: &str) -> (Pubkey, u8) {
    let seeds = &[b"instance", instance_name.as_bytes()];
    Pubkey::find_program_address(seeds, program_id)
}

fn verify_pda(
    program_id: &Pubkey,
    seed: &str,
    pda: &Pubkey,
    pda_bump_seed: u8,
    make_pda_fn: impl Fn(&Pubkey, &str) -> (Pubkey, u8),
) {
    let (expected_pda, expected_pda_bump_seed) = make_pda_fn(program_id, seed);
    assert_eq!(pda, &expected_pda);
    assert_eq!(pda_bump_seed, expected_pda_bump_seed);
}

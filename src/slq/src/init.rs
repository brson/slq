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
    instr: SlqInitializeInstanceInstruction,
) -> ProgramResult {
    match instr {
        SlqInitializeInstanceInstruction::Init(instr) => instr.exec(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqInitializeInstanceInstruction {
    Init(Init),
}

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: instance_pda - pda, writable, owner=system_program, uninitialized
/// - 2: system_program - executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Init {
    lamports: u64,
    instance_name: String,
    approval_threshold: u8,
    admin_accounts: Vec<Pubkey>,
    instance_pda_bump_seed: u8,
}

impl Init {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        lamports: u64,
        instance_name: String,
        approval_threshold: u8,
        admin_accounts: Vec<Pubkey>,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr = Init {
            lamports,
            instance_name,
            approval_threshold,
            admin_accounts,
            instance_pda_bump_seed,
        };

        instr.validate()?;

        let instr = SlqInstruction::InitializeInstance(SlqInitializeInstanceInstruction::Init(instr));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(instance_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let rent_payer = next_account_info(accounts_iter)?;
        let instance_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        {
            assert!(rent_payer.is_writable);
            assert!(rent_payer.is_signer);
            assert!(instance_pda.is_writable);
            assert_eq!(instance_pda.owner, system_program.key,
                       "instance_pda is not owned by the system program");
            assert_eq!(system_program.key, &system_program::ID,
                       "unexpected system program id");
            assert!(system_program.executable);

            verify_pda(
                program_id,
                &self.instance_name,
                instance_pda.key,
                self.instance_pda_bump_seed,
                make_instance_pda,
            );
        }

        let instance_size = get_instance_packed_len(&SlqInstance::default())?;
        let rent = Rent::get()?;
        let needed_lamports = rent.minimum_balance(instance_size);

        if self.lamports < needed_lamports {
            msg!("Instance_pda does not have the enough lamports");
            return Err(ProgramError::InsufficientFunds);
        }

        let space = instance_size.try_into().unwrap();
        invoke_signed(
            &system_instruction::create_account(
                rent_payer.key,
                instance_pda.key,
                self.lamports,
                space,
                program_id,
            ),
            &[rent_payer.clone(), instance_pda.clone()],
            &[&[
                b"instance",
                self.instance_name.as_ref(),
                &[self.instance_pda_bump_seed],
            ]],
        )?;

        msg!("init instance_pda's data");

        Ok(self.serialize(&mut &mut instance_pda.data.borrow_mut()[..])?)
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
    // todo: better error messages
    let (expected_pda, expected_pda_bump_seed) = make_pda_fn(program_id, seed);
    assert_eq!(pda, &expected_pda);
    assert_eq!(pda_bump_seed, expected_pda_bump_seed);
}

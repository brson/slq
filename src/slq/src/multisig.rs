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

use crate::state::MultisigConfigInstance;
use crate::SlqInstruction;

pub const MAX_MULTISIG_OWNERS: usize = 5;

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqMultisigInstruction,
) -> ProgramResult {
    match instr {
        SlqMultisigInstruction::Init(instr) => instr.exec(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqMultisigInstruction {
    Init(Init),
}

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: instance_pda - pda, writable, uninitialized
/// - 2: system_program - executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Init {
    instance_name: String,
    approval_threshold: u8,
    owners: Vec<Pubkey>,
    instance_pda_bump_seed: u8,
}

impl Init {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        instance_name: String,
        approval_threshold: u8,
        owners: Vec<Pubkey>,
    ) -> Result<Instruction> {
        let (instance_pda, instance_pda_bump_seed) = make_instance_pda(program_id, &instance_name);

        let instr = Init {
            instance_name,
            approval_threshold,
            owners,
            instance_pda_bump_seed,
        };

        instr.validate()?;

        let instr = SlqInstruction::Multisig(SlqMultisigInstruction::Init(instr));

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
            let instance_pda_initialized = {
                instance_pda.owner != system_program.key
                    || **instance_pda.lamports.borrow() > 0
                    || instance_pda.data.borrow().len() > 0
            };
            if instance_pda_initialized {
                msg!("instance_pda has already been initialized");
                return Err(ProgramError::AccountAlreadyInitialized);
            }
            assert_eq!(
                system_program.key,
                &system_program::ID,
                "unexpected system program id"
            );
            assert!(system_program.executable);

            verify_pda(
                program_id,
                &self.instance_name,
                instance_pda.key,
                self.instance_pda_bump_seed,
                make_instance_pda,
            );
        }

        let instance_size = get_instance_packed_len(&MultisigConfigInstance::default())?;
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(instance_size);

        if **rent_payer.lamports.borrow() < rent_lamports {
            msg!("rent_payer does not have the enough lamports to pay instance rent");
            return Err(ProgramError::InsufficientFunds);
        }

        let space = instance_size.try_into().unwrap(); // error handling
        invoke_signed(
            &system_instruction::create_account(
                rent_payer.key,
                instance_pda.key,
                rent_lamports,
                space,
                program_id,
            ),
            &[rent_payer.clone(), instance_pda.clone()],
            &[&[
                b"multisig-instance",
                self.instance_name.as_ref(),
                &[self.instance_pda_bump_seed],
            ]],
        )?;

        let instance = MultisigConfigInstance {
            approval_threshold: self.approval_threshold,
            owners: create_owners_array(&self.owners),
        };

        let instance_pda_data = &mut *instance_pda.data.borrow_mut();

        instance.serialize(instance_pda_data)?;

        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.approval_threshold == 0 {
            bail!("approval threshold must be greater than 0");
        }

        if self.owners.is_empty() {
            bail!("must have greater than 0 owners");
        }

        if self.owners.len() > MAX_MULTISIG_OWNERS {
            bail!(
                "number of owners must be not be greater than {}",
                MAX_MULTISIG_OWNERS
            );
        }

        if usize::from(self.approval_threshold) > self.owners.len() {
            bail!("approval threshold must not be greater than number of owners");
        }

        let mut sorted_accounts = self.owners.clone();
        sorted_accounts.sort_unstable();
        sorted_accounts.dedup();

        if sorted_accounts.len() != self.owners.len() {
            bail!("must not have duplicate owners");
        }

        Ok(())
    }
}

pub fn make_instance_pda(program_id: &Pubkey, instance_name: &str) -> (Pubkey, u8) {
    let seeds = &[b"multisig-instance", instance_name.as_bytes()];
    Pubkey::find_program_address(seeds, program_id)
}

pub fn verify_pda(
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

pub fn create_owners_array(accounts: &[Pubkey]) -> [Pubkey; MAX_MULTISIG_OWNERS] {
    let mut array = [Pubkey::default(); MAX_MULTISIG_OWNERS];
    assert!(accounts.len() <= array.len());
    for (i, account) in accounts.iter().enumerate() {
        array[i] = *account;
    }

    array
}

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
    instr: SlqVaultInstruction,
) -> ProgramResult {
    match instr {
        SlqVaultInstruction::CreateVault(instr) => instr.exec(program_id, accounts),
        SlqVaultInstruction::DepositToVault(instr) => instr.exec(program_id, accounts),
        SlqVaultInstruction::WithdrawFromVault(instr) => instr.exec(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqVaultInstruction {
    CreateVault(CreateVault),
    DepositToVault(DepositToVault),
    WithdrawFromVault(WithdrawFromVault),
}

/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program_id
/// - 2: system_program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateVault {
    pub vault_name: String,
    pub vault_bump_seed: u8,
}

/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program_id
/// - 2: system_program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositToVault {
    pub vault_name: String,
    pub vault_bump_seed: u8,
    pub amount: u64,
}

/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program
/// - 2: system_program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct WithdrawFromVault {
    pub vault_name: String,
    pub vault_bump_seed: u8,
    pub amount: u64,
}

impl CreateVault {
    pub fn build_instruction(
        program_id: &Pubkey,
        payer: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction> {
        let (vault_pubkey, vault_bump_seed) = vault_pda(program_id, payer, vault_name);

        let slq_instruction =
            SlqInstruction::Vault(SlqVaultInstruction::CreateVault(CreateVault {
                vault_name: vault_name.to_string(),
                vault_bump_seed,
            }));
        //        let mut slq_data: Vec<u8> = Vec::new();
        //        slq_instruction.serialize(&mut slq_data)
        //            .map_err(|_| anyhow!("unable to serialize instruction"))?;

        let accounts = vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(
            *program_id,
            &slq_instruction,
            accounts,
        ))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let vault = next_account_info(accounts_iter)?;

        assert!(payer.is_signer);
        assert!(payer.is_writable);
        // todo vault asserts

        let (vault_, vault_bump_seed_) = vault_pda(program_id, payer.key, &self.vault_name);
        assert_eq!(vault.key, &vault_);
        assert_eq!(self.vault_bump_seed, vault_bump_seed_);

        let lamports = 1000; // todo

        invoke_signed(
            &system_instruction::create_account(payer.key, vault.key, lamports, 0, program_id),
            &[payer.clone(), vault.clone()],
            &[&[
                b"vault",
                self.vault_name.as_ref(),
                payer.key.as_ref(),
                &[self.vault_bump_seed],
            ]],
        )?;

        Ok(())
    }
}

impl DepositToVault {
    pub fn build_instruction(
        program_id: &Pubkey,
        payer: &Pubkey,
        vault_name: &str,
        amount: u64,
    ) -> Result<Instruction> {
        let (vault_pubkey, vault_bump_seed) = vault_pda(program_id, payer, vault_name);

        let slq_instruction =
            SlqInstruction::Vault(SlqVaultInstruction::DepositToVault(DepositToVault {
                vault_name: vault_name.to_string(),
                vault_bump_seed,
                amount,
            }));

        let accounts = vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(
            *program_id,
            &slq_instruction,
            accounts,
        ))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let vault = next_account_info(accounts_iter)?;

        // todo validation

        invoke_signed(
            &system_instruction::transfer(payer.key, vault.key, self.amount),
            &[payer.clone(), vault.clone()],
            &[&[
                b"vault",
                self.vault_name.as_ref(),
                payer.key.as_ref(),
                &[self.vault_bump_seed],
            ]],
        )?;

        Ok(())
    }
}

impl WithdrawFromVault {
    pub fn build_instruction(
        program_id: &Pubkey,
        payer: &Pubkey,
        vault_name: &str,
        amount: u64,
    ) -> Result<Instruction> {
        let (vault_pubkey, vault_bump_seed) = vault_pda(program_id, payer, vault_name);

        let slq_instruction =
            SlqInstruction::Vault(SlqVaultInstruction::WithdrawFromVault(WithdrawFromVault {
                vault_name: vault_name.to_string(),
                vault_bump_seed,
                amount,
            }));

        let accounts = vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(
            *program_id,
            &slq_instruction,
            accounts,
        ))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let vault = next_account_info(accounts_iter)?;

        // todo validation
        // todo:
        // - if pda_vault has enough balance
        // - if the caller is the one who created the vault
        invoke_signed(
            &system_instruction::transfer(vault.key, payer.key, self.amount),
            &[payer.clone(), vault.clone()],
            &[&[
                b"vault",
                self.vault_name.as_ref(),
                payer.key.as_ref(),
                &[self.vault_bump_seed],
            ]],
        )?;

        Ok(())
    }
}

fn vault_pda(program_id: &Pubkey, payer: &Pubkey, name: &str) -> (Pubkey, u8) {
    let vault_seeds = &[b"vault", name.as_bytes(), payer.as_ref()];
    let (vault, vault_bump_seed) = Pubkey::find_program_address(vault_seeds, program_id);

    (vault, vault_bump_seed)
}

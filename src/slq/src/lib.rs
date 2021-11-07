#![allow(unused)]

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
use std::convert::{TryFrom, TryInto};

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use solana_program::entrypoint;
    use super::process_instruction;
    entrypoint!(process_instruction);
}

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("process instruction");

    let mut instruction_data = instruction_data;
    let instr = Instruction::deserialize(&mut instruction_data)?;
    let accounts_iter = &mut accounts.iter();

    match instr {
        Instruction::CreateVault(instr) => {
            let payer = next_account_info(accounts_iter)?;
            let vault = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;
            instr.exec(program_id, payer, vault, system_program)?;
        }
        Instruction::DepositToVault(instr) => {
            let payer = next_account_info(accounts_iter)?;
            let vault = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;
            instr.exec(program_id, payer, vault, system_program)?;
        }
    }

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Instruction {
    CreateVault(CreateVault),
    DepositToVault(DepositToVault),
}


/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program?
/// - 2: system_program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateVault {
    pub vault_bump_seed: u8,
}

/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program
/// - 2: system_program: executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositToVault {
    pub vault_bump_seed: u8,
    pub amount: u64,
}

impl CreateVault {
    fn exec<'accounts>(
        &self,
        program_id: &Pubkey,
        payer: &AccountInfo<'accounts>,
        vault: &AccountInfo<'accounts>,
        system_program: &AccountInfo) -> ProgramResult
    {
        assert!(payer.is_signer);
        assert!(payer.is_writable);
        // todo vault asserts
        assert!(system_program.executable);

        let (vault_, vault_bump_seed_) = vault_pda(program_id, payer.key);
        assert_eq!(vault.key, &vault_);
        assert_eq!(self.vault_bump_seed, vault_bump_seed_);

        let lamports = 1000; // todo

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                vault.key,
                lamports,
                0,
                program_id,
            ),
            &[
                payer.clone(),
                vault.clone(),
            ],
            &[
                &[
                    b"vault",
                    payer.key.as_ref(),
                    &[self.vault_bump_seed],
                ],
            ]
        )?;

        Ok(())
    }
}

impl DepositToVault {
    fn exec(
        &self,
        program_id: &Pubkey,
        payer: &AccountInfo,
        vault: &AccountInfo,
        system_program: &AccountInfo) -> ProgramResult
    {
        todo!()
    }
}

pub fn vault_pda(program_id: &Pubkey, payer: &Pubkey) -> (Pubkey, u8) {
    let vault_seeds = &[b"vault", payer.as_ref()];
    let (vault, vault_bump_seed) = Pubkey::find_program_address(vault_seeds, program_id);

    (vault, vault_bump_seed)
}

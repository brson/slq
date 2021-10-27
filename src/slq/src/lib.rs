#![allow(unused)]

use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info, next_account_infos},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
};
use std::convert::{TryFrom, TryInto};

entrypoint!(process_instruction);

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
            instr.exec(program_id, payer, vault)?;
        }
        Instruction::DepositToVault(instr) => {
            let payer = next_account_info(accounts_iter)?;
            let vault = next_account_info(accounts_iter)?;
            instr.exec(program_id, payer, vault)?;
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
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateVault {
    pub vault_bump_seed: u8,
}

/// # Accounts
///
/// - 0: payer: signer, writable
/// - 1: vault: pda, writable, owner=program
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositToVault {
    pub vault_bump_seed: u8,
    pub amount: u64,
}

impl CreateVault {
    fn exec(&self, program_id: &Pubkey, payer: &AccountInfo, vault: &AccountInfo) -> ProgramResult {
        assert!(payer.is_signer);
        assert!(payer.is_writable);
        // todo vault asserts

        let vault_seeds = &[b"vault", payer.key.as_ref()];
        let (vault_, vault_bump_seed_) = Pubkey::find_program_address(vault_seeds, program_id);

        todo!()
    }
}

impl DepositToVault {
    fn exec(&self, program_id: &Pubkey, payer: &AccountInfo, vault: &AccountInfo) -> ProgramResult {
        todo!()
    }
}

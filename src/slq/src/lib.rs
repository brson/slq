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

            todo!()
        }
        Instruction::DepositToVault(instr) => {
            let payer = next_account_info(accounts_iter)?;
            let vault = next_account_info(accounts_iter)?;

            todo!()
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Instruction {
    CreateVault(CreateVault),
    DepositToVault(DepositToVault),
}


/// # Accounts
///
/// - 0: payer: signer, writeable
/// - 1: vault: pda, writeable, owner=program?
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateVault {
    vault_bump_seed: u8,
}

/// # Accounts
///
/// - 0: payer: signer, writeable
/// - 1: vault: pda, writeable, owner=program
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DepositToVault {
    vault_bump_seed: u8,
    amount: u64,
}



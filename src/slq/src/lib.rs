#![allow(unused)]

use anyhow::{anyhow, Result};
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

pub mod admin;
pub mod init;
pub mod state;
pub mod vault;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use super::process_instruction;
    use solana_program::entrypoint;
    entrypoint!(process_instruction);
}

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("process instruction");

    let mut instruction_data = instruction_data;
    let instr = SlqInstruction::deserialize(&mut instruction_data)?;

    match instr {
        SlqInstruction::InitializeInstance(instr) => {
            init::exec(program_id, accounts, instr)
        }
        SlqInstruction::Admin(instr) => {
            admin::exec(program_id, accounts, instr)
        }
        SlqInstruction::Vault(instr) => {
            vault::exec(program_id, accounts, instr)
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqInstruction {
    InitializeInstance(init::SlqInitializeInstanceInstruction),
    Admin(admin::SlqAdminInstruction),
    Vault(vault::SlqVaultInstruction),
}


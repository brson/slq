#![allow(unused)]

use crate::SlqInstruction;
use anyhow::{anyhow, bail, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::get_instance_packed_len;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::nonce::State;
use solana_program::system_instruction::assign;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
};
use std::convert::{TryFrom, TryInto};

pub fn exec(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instr: SlqNonceInstruction,
) -> ProgramResult {
    match instr {
        SlqNonceInstruction::Withdraw(instr) => instr.exec(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SlqNonceInstruction {
    Withdraw(WithdrawNonceAccount),
}

/// # Accounts
///
/// - 0: rent_payer - writable, signer
/// - 1: nonce_account - writable, initialized
/// - 2: system_program - executable
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct WithdrawNonceAccount {
    nonce_pubkey: Pubkey,
}

impl WithdrawNonceAccount {
    pub fn build_instruction(
        program_id: &Pubkey,
        rent_payer: &Pubkey,
        nonce_account: &Pubkey,
    ) -> Result<Instruction> {
        let instr = SlqInstruction::Nonce(SlqNonceInstruction::Withdraw(WithdrawNonceAccount {
            nonce_pubkey: *nonce_account,
        }));

        let accounts = vec![
            AccountMeta::new(*rent_payer, true),
            AccountMeta::new(*nonce_account, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];

        Ok(Instruction::new_with_borsh(*program_id, &instr, accounts))
    }

    fn exec(&self, program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        // change the nonce account owner and resize, then withdraw nonce account

        let accounts_iter = &mut accounts.iter();

        let rent_payer = next_account_info(accounts_iter)?;
        let nonce_account = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        {
            assert!(rent_payer.is_writable);
            assert!(rent_payer.is_signer);
            assert!(nonce_account.is_writable);

            // todo if nonce account is initialized
            assert_eq!(
                nonce_account.owner, system_program.key,
                "nonce_account isn't owned by system program"
            );
            assert_eq!(
                system_program.key,
                &system_program::ID,
                "unexpected system program id"
            );
            assert!(system_program.executable);
        }
        // todo: authorize_nonce_account

        /*
        invoke(
            &system_instruction::assign(&self.pubkey, program_id),
            &[rent_payer.clone()],
        )?;
         */

        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(State::size());

        msg!("self: {:?}", self);

        invoke(
            &system_instruction::withdraw_nonce_account(
                &self.nonce_pubkey,
                rent_payer.key,
                rent_payer.key,
                rent_lamports,
            ),
            &[rent_payer.clone()],
        )?;

        Ok(())
    }
}

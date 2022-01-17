#![allow(unused)]

use anyhow::{anyhow, bail, Context, Result};
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::str::FromStr;
use structopt::StructOpt;

use borsh::BorshDeserialize;
use slq::admin::{AddAdminAccountAdmin, ChangeApprovalThresholdAdmin, RemoveAdminAccountAdmin};
use slq::init;
use slq::init::make_instance_pda;
use slq::state::{AdminConfig, SlqInstance, MAX_ADMIN_ACCOUNTS};

// todo: follow same pattern as in admin
#[derive(StructOpt, Debug)]
pub enum VaultCommand {
    CreateVault,
    DepositToVault { amount: u64 },
    WithdrawFromVault { amount: u64 }, // todo: withdraw-all command
}

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: VaultCommand,
) -> Result<Instruction> {
    let vault_name = "vault".to_string();
    match cmd {
        VaultCommand::CreateVault => {
            slq::vault::CreateVault::build_instruction(program_id, rent_payer, &vault_name)
        }
        VaultCommand::DepositToVault { amount } => slq::vault::DepositToVault::build_instruction(
            program_id,
            rent_payer,
            &vault_name,
            amount,
        ),
        VaultCommand::WithdrawFromVault { amount } => {
            slq::vault::WithdrawFromVault::build_instruction(
                program_id,
                rent_payer,
                &vault_name,
                amount,
            )
        }
    }
}

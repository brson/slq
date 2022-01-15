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
use slq::admin::ChangeApprovalThresholdAdmin;
use slq::init;
use slq::init::make_instance_pda;
use slq::state::AdminConfig;
use slq::state::SlqInstance;

#[derive(StructOpt, Debug)]
pub enum AdminCommand {
    ChangeApprovalThreshold(ChangeApprovalThresholdAdminCommand),
    AddAdminAccount(AddAdminAccountAdminCommand),
    RemoveAdminAccount(RemoveAdminAccountAdminCommand),
}

#[derive(StructOpt, Debug)]
pub struct ChangeApprovalThresholdAdminCommand {
    instance_name: String,
    approval_threshold: u8,
}

#[derive(StructOpt, Debug)]
pub struct AddAdminAccountAdminCommand {
    instance_name: String,
    account: String,
}

#[derive(StructOpt, Debug)]
pub struct RemoveAdminAccountAdminCommand {
    instance_name: String,
    account: String,
}

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: AdminCommand,
) -> Result<Instruction> {
    match cmd {
        AdminCommand::ChangeApprovalThreshold(cmd) => {
            let (instance_pubkey, _) = make_instance_pda(program_id, &cmd.instance_name);
            let instance_account = client.get_account(&instance_pubkey)?;
            let slq_instance = SlqInstance::try_from_slice(&instance_account.data)?;

            let admin_accounts = slq_instance
                .admin_config
                .admin_accounts
                .iter()
                .filter(|account| **account != Pubkey::default())
                .copied()
                .collect::<Vec<Pubkey>>();

            let cmd_approval_threshold = usize::from(cmd.approval_threshold);
            if cmd_approval_threshold > admin_accounts.len() {
                bail!(
                    "approval threshold must be less than or equal to {}, the number of administrators",
                    admin_accounts.len()
                );
            }
            if cmd_approval_threshold == admin_accounts.len() {
                bail!("approval threshold is {} already", cmd.approval_threshold);
            }

            ChangeApprovalThresholdAdmin::build_instruction(
                program_id,
                rent_payer,
                cmd.instance_name,
                cmd.approval_threshold,
            )
        }
        AdminCommand::AddAdminAccount(cmd) => todo!(),
        AdminCommand::RemoveAdminAccount(cmd) => todo!(),
    }
}

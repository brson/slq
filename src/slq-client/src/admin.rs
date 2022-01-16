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
            change_approval_threshold_instruction(client, program_id, rent_payer, cmd)
        }
        AdminCommand::AddAdminAccount(cmd) => {
            add_admin_account_instruction(client, program_id, rent_payer, cmd)
        }
        AdminCommand::RemoveAdminAccount(cmd) => {
            remove_admin_account_instruction(client, program_id, rent_payer, cmd)
        }
    }
}

fn change_approval_threshold_instruction(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: ChangeApprovalThresholdAdminCommand,
) -> Result<Instruction> {
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
    {
        let cmd_approval_threshold = usize::from(cmd.approval_threshold);
        if cmd_approval_threshold == 0 {
            bail!("approval threshold must be greater than 0");
        }
        if cmd_approval_threshold > admin_accounts.len() {
            bail!(
                "approval threshold must be less than or equal to {}, the number of administrators",
                admin_accounts.len()
            );
        }
        if cmd_approval_threshold == admin_accounts.len() {
            bail!("approval threshold is {} already", cmd.approval_threshold);
        }
    }

    ChangeApprovalThresholdAdmin::build_instruction(
        program_id,
        rent_payer,
        cmd.instance_name,
        cmd.approval_threshold,
    )
}

fn add_admin_account_instruction(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: AddAdminAccountAdminCommand,
) -> Result<Instruction> {
    let (instance_pubkey, _) = make_instance_pda(program_id, &cmd.instance_name);
    let instance_account = client.get_account(&instance_pubkey)?;
    let slq_instance = SlqInstance::try_from_slice(&instance_account.data)?;

    let new_admin_account = Pubkey::from_str(&cmd.account)?;
    let mut admin_accounts = slq_instance
        .admin_config
        .admin_accounts
        .iter()
        .filter(|account| **account != Pubkey::default())
        .copied()
        .collect::<Vec<Pubkey>>();

    if admin_accounts.len() == MAX_ADMIN_ACCOUNTS {
        bail!(
            "there are already {} admin accounts, remove one to add a new account",
            MAX_ADMIN_ACCOUNTS
        );
    }
    if admin_accounts.contains(&new_admin_account) {
        bail!("account {} already exists", &new_admin_account);
    }

    AddAdminAccountAdmin::build_instruction(
        program_id,
        rent_payer,
        cmd.instance_name,
        new_admin_account,
    )
}

fn remove_admin_account_instruction(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: RemoveAdminAccountAdminCommand,
) -> Result<Instruction> {
    let (instance_pubkey, _) = make_instance_pda(program_id, &cmd.instance_name);
    let instance_account = client.get_account(&instance_pubkey)?;
    let slq_instance = SlqInstance::try_from_slice(&instance_account.data)?;

    let to_remove_admin_account = Pubkey::from_str(&cmd.account)?;
    let mut admin_accounts = slq_instance
        .admin_config
        .admin_accounts
        .iter()
        .filter(|account| **account != Pubkey::default())
        .copied()
        .collect::<Vec<Pubkey>>();

    if !admin_accounts.contains(&to_remove_admin_account) {
        bail!("account {} isn't in the admin list", &to_remove_admin_account);
    }
    if admin_accounts.len() == 1 {
        bail!("must have at least 1 admin account, add a new admin account before remove the current one");
    }
    if admin_accounts.len() == usize::from(slq_instance.admin_config.approval_threshold) {
        bail!("approval threshold is the same as the number of admin accounts, change the approval threshold before remove an account");
    }

    RemoveAdminAccountAdmin::build_instruction(
        program_id,
        rent_payer,
        cmd.instance_name,
        to_remove_admin_account,
    )
}

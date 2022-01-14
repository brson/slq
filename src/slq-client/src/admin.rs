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

use slq::init;
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
        
        _ => todo!(),
    }
}

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

use super::InitializeInstanceCommand;

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: InitializeInstanceCommand,
) -> Result<Instruction> {
    let admin_accounts = cmd
        .admin_accounts
        .iter()
        .map(|account| Pubkey::from_str(account))
        .collect::<Result<Vec<Pubkey>, _>>()?;

    init::Init::build_instruction(
        program_id,
        rent_payer,
        cmd.instance_name,
        cmd.approval_threshold,
        admin_accounts,
    )
}

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
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

use borsh::BorshDeserialize;
use slq::init;
use slq::init::make_instance_pda;
use slq::state::{AdminConfig, SlqInstance, MAX_ADMIN_ACCOUNTS};

#[derive(StructOpt, Debug)]
pub enum MultisigCommand {
    /// Create the multisig admins on-chain.
    Init(Init),

    /// Begin a multisig transaction.
    ///
    /// Loads a transaction from file.
    /// Creates a temporary nonce account on-chain,
    /// assigning authority to the payer.
    /// Sign the transaction, write to file.
    StartTransaction(StartTransaction),

    /// Load a transaction from file, sign it,
    /// write to file.
    SignTransaction(SignTransaction),

    /// Load a transaction from file,
    /// submit it to the network.
    ExecTransaction(ExecTransaction),

    /// Withdraw funds from nonce account.
    ///
    /// Must be performed by the account that
    /// started the transaction.
    CancelTransaction(CancelTransaction),

    /// Starts a multisig transaction to destroy the multisig instance.
    StartDestroy(StartDestroy),
}

#[derive(StructOpt, Debug)]
pub struct Init {
    /// The name of the multisig instance.
    ///
    /// There can be multiple multisig instances per multisig program.
    instance_name: String,
    approval_threshold: u8,
    admin_accounts: Vec<String>,
}

#[derive(StructOpt, Debug)]
pub struct StartTransaction {
    instance_name: String,
    /// Used to identify the nonce account.
    transaction_name: String,
    transaction_path: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct SignTransaction {
    transaction_path: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct ExecTransaction {
    transaction_path: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct CancelTransaction {
    transaction_path: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct StartDestroy {
    instance_name: String,
}

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: MultisigCommand,
) -> Result<Instruction> {
    todo!()
}

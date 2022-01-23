#![allow(unused)]

use anyhow::{anyhow, bail, Context, Result};
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::message::SanitizedMessage;
use solana_sdk::nonce::State;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::signers::Signers;
use solana_sdk::system_instruction;
use solana_sdk::sysvar::instructions::construct_instructions_data;
use solana_sdk::transaction::Transaction;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

use borsh::BorshDeserialize;
use slq::state::{AdminConfig, SlqInstance, MAX_ADMIN_ACCOUNTS};
use solana_sdk::borsh::get_instance_packed_len;

use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(StructOpt, Debug)]
pub enum MultisigCommand {
    /// Create the multisig admins on-chain.
    Init(Init),
}

#[derive(StructOpt, Debug)]
pub struct Init {
    /// The name of the multisig instance.
    ///
    /// There can be multiple multisig instances per multisig program.
    instance_name: String,
    approval_threshold: u8,
    owners: Vec<String>,
}

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Keypair,
    cmd: MultisigCommand,
) -> Result<Instruction> {
    match cmd {
        MultisigCommand::Init(cmd) => cmd.exec(client, program_id, &rent_payer.pubkey()),
        _ => panic!(),
    }
}

impl Init {
    fn exec(
        &self,
        client: &RpcClient,
        program_id: &Pubkey,
        rent_payer: &Pubkey,
    ) -> Result<Instruction> {
        let owners = self
            .owners
            .iter()
            .map(|owner| Pubkey::from_str(owner))
            .collect::<Result<Vec<Pubkey>, _>>()?;

        slq::multisig::Init::build_instruction(
            program_id,
            rent_payer,
            self.instance_name.clone(),
            self.approval_threshold,
            owners,
        )
    }
}

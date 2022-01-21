#![allow(unused)]

use anyhow::{anyhow, bail, Context, Result};
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::signers::Signers;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

use borsh::BorshDeserialize;
use slq::state::{AdminConfig, SlqInstance, MAX_ADMIN_ACCOUNTS};

use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};

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

    /// For test: create a `Transaction` and save it to disk
    DemoTransaction(DemoTransaction),
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

#[derive(StructOpt, Debug)]
pub struct DemoTransaction {
    transaction_path: PathBuf,
}

pub(crate) fn do_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: MultisigCommand,
) -> Result<Instruction> {
    match cmd {
        MultisigCommand::Init(cmd) => cmd.exec(client, program_id, rent_payer),
        MultisigCommand::StartTransaction(cmd) => cmd.exec(client, program_id, rent_payer),
        MultisigCommand::DemoTransaction(cmd) => cmd.exec(client, program_id, rent_payer),
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

impl StartTransaction {
    fn exec(
        &self,
        client: &RpcClient,
        program_id: &Pubkey,
        rent_payer: &Pubkey,
    ) -> Result<Instruction> {
        let nonce_account = Keypair::new();

        let tx = load_tx(&self.transaction_path)?;
        println!("load_tx {:#?}", tx);

        let mut signatures = tx.signatures;
        let mut user_instr_list = vec![];

        for i in tx.message.instructions {
            let decompiled_instr =
                Message::deserialize_instruction(i.program_id_index.into(), &i.data)?;
            user_instr_list.push(decompiled_instr);
        }

        let mut new_instr_list = vec![system_instruction::advance_nonce_account(
            &nonce_account.pubkey(),
            rent_payer,
        )];

        new_instr_list.append(&mut user_instr_list);

        // todo: get rent lamports for creating nonce_account
        let lamports = 1024;
        new_instr_list.push(system_instruction::withdraw_nonce_account(
            &nonce_account.pubkey(),
            rent_payer,
            rent_payer,
            lamports,
        ));

        let message = Message::new_with_nonce(
            new_instr_list,
            Some(rent_payer),
            &nonce_account.pubkey(),
            rent_payer,
        );

        let mut new_tx = Transaction::new_unsigned(message);

        // todo: create_nonce_account on chain and get the nonce for `recent_blockhash`
        let hash = client.get_recent_blockhash()?.0;

        // todo: sign with rent_payer's keypair
        let signers: Vec<&dyn Signer> = vec![&nonce_account];
        new_tx.try_sign(&signers, hash)?; 

        println!("new_tx {:#?}", new_tx);
        write_tx_to_file(&self.transaction_path, &new_tx)?;

        // for on-chain file
        // set authority to rent_payer

        // instruction for creating nonce-account
        // rent_payer, singer, payer
        // nonce_keypair, signer
        // system_program

        // submit to on-chain program for exec
        // or call system_program directly

        // temporarily return `Instruction`
        Ok(system_instruction::advance_nonce_account(
            &nonce_account.pubkey(),
            rent_payer,
        ))
    }
}

impl DemoTransaction {
    fn exec(
        &self,
        client: &RpcClient,
        program_id: &Pubkey,
        rent_payer: &Pubkey,
    ) -> Result<Instruction> {
        let owners = vec![Pubkey::new_unique()];

        let instr = slq::admin::ChangeApprovalThresholdAdmin::build_instruction(
            program_id,
            rent_payer,
            "bar1".to_string(),
            1,
        )?;

        let return_copy_instr = instr.clone();
        let tx = Transaction::new_with_payer(&[instr], Some(rent_payer));

        write_tx_to_file(&self.transaction_path, &tx)?;

        // temporarily return `Instruction`
        Ok(return_copy_instr)
    }
}

fn write_tx_to_file<P: AsRef<Path>>(path: P, tx: &Transaction) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, tx).map_err(|e| anyhow!("{}", e))
}

fn load_tx<P: AsRef<Path>>(path: P) -> Result<Transaction> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let tx = serde_json::from_reader(reader)?;

    Ok(tx)
}

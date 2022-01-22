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
use solana_sdk::sysvar::instructions::construct_instructions_data;
use solana_sdk::message::SanitizedMessage;
use solana_sdk::hash::Hash;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

use borsh::BorshDeserialize;
use solana_sdk::borsh::get_instance_packed_len;
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

impl StartTransaction {
    pub fn exec(
        &self,
        client: &RpcClient,
        program_id: &Pubkey,
        rent_payer: &Keypair,
    ) -> Result<()> {
        let nonce_account = Keypair::new();
        println!("nonce_account_pubkey: {:#?}", nonce_account.pubkey());
        
        let tx = load_tx(&self.transaction_path)?;

        let mut signatures = tx.signatures;
        let mut user_instr_list = vec![];

        let instr_num = tx.message.instructions.len();
        let msg = SanitizedMessage::try_from(tx.message)?;
        let msg = construct_instructions_data(&msg);
        
        for i in 0..instr_num {
            let decompiled_instr =
                Message::deserialize_instruction(i, &msg)?;
            user_instr_list.push(decompiled_instr);
        }

//        println!("user_instr: {:#?}", user_instr_list);
        let mut new_instr_list = vec![system_instruction::advance_nonce_account(
            &nonce_account.pubkey(),
            &rent_payer.pubkey(),
        )];

        new_instr_list.append(&mut user_instr_list);

        // todo: lamports caculation isn't correct
        let account_size = get_instance_packed_len(&nonce_account.pubkey())?;
        let lamports = client.get_minimum_balance_for_rent_exemption(account_size)?;
        let lamports = 1447680;

        new_instr_list.push(system_instruction::withdraw_nonce_account(
            &nonce_account.pubkey(),
            &rent_payer.pubkey(),
            &rent_payer.pubkey(),
            lamports,
        ));

        let signers: Vec<&dyn Signer> = vec![&nonce_account, rent_payer];

        // build on-chain tx
        let onchain_instr = system_instruction::create_nonce_account(
            &rent_payer.pubkey(),
            &nonce_account.pubkey(),
            &rent_payer.pubkey(),
            lamports,
        );
        let mut onchain_tx = Transaction::new_with_payer(
            &onchain_instr,
            Some(&rent_payer.pubkey()),
        );
        
        onchain_tx.try_sign(&signers, client.get_latest_blockhash()?)?;

//        println!("onchain_tx {:#?}", onchain_tx);
        
        let sig = client.send_and_confirm_transaction(&onchain_tx)?;

        println!("hello, sig {:#?}", sig);

        // build off-chain tx
        let mut new_tx = Transaction::new_with_payer(
            &new_instr_list,
            Some(&rent_payer.pubkey()),
        );
        println!("new_tx {:#?}", new_tx);
        
        let onchain_nonce_account = client.get_account(&nonce_account.pubkey())?;
        println!("get_account: {:#?}", onchain_nonce_account);
        
//        let hash = Hash::new(&onchain_nonce_account.data);
        let hash: Hash = onchain_nonce_account.deserialize_data()?;
        println!("hash: {:?}", hash);
        println!("blockhash: {:#?}", client.get_latest_blockhash()?);

        let signers: Vec<&dyn Signer> = vec![&nonce_account, rent_payer];
        println!("signers: {:#?}", signers);
        new_tx.try_sign(&signers, hash)?; 

        println!("new_tx_signed {:#?}", new_tx);
        write_tx_to_file(&self.transaction_path, &new_tx)?;

        println!("hi");

        Ok(())
    }
}

impl DemoTransaction {
    pub fn exec(
        &self,
        client: &RpcClient,
        program_id: &Pubkey,
        rent_payer: &Pubkey,
    ) -> Result<()> {
        let instr = slq::admin::ChangeApprovalThresholdAdmin::build_instruction(
            program_id,
            rent_payer,
            "foo".to_string(),
            1,
        )?;

        let tx = Transaction::new_with_payer(&[instr], Some(rent_payer));

        write_tx_to_file(&self.transaction_path, &tx)?;

        Ok(())
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

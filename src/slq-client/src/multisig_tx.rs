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
pub enum MultisigTxCommand {
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
    payer: &Keypair,
    cmd: MultisigTxCommand,
) -> Result<()> {
    match cmd {
        MultisigTxCommand::StartTransaction(cmd) => cmd.exec(client, program_id, payer),
        MultisigTxCommand::SignTransaction(cmd) => cmd.exec(program_id, payer),
        MultisigTxCommand::ExecTransaction(cmd) => cmd.exec(client, program_id, payer),
        MultisigTxCommand::DemoTransaction(cmd) => {
            cmd.exec(client, program_id, &payer.pubkey())
        }
        _ => todo!(),
    }
}


impl StartTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Keypair) -> Result<()> {
        let nonce_account = Keypair::new();
        let nonce_account_pubkey = nonce_account.pubkey();
        let rent_payer_pubkey = rent_payer.pubkey();

        // for testing
        fs::write(&PathBuf::from("nonce_pubkey"), nonce_account.to_bytes())?;
        println!("nonce_keypair saved");
        
        // load and decompile offchain tx file
        let tx = load_tx(&self.transaction_path)?;

        let mut instr_offchain = vec![];

        let instr_num = tx.message.instructions.len();
        let message = SanitizedMessage::try_from(tx.message)?;
        let message = construct_instructions_data(&message);

        for i in 0..instr_num {
            let decompiled_instr = Message::deserialize_instruction(i, &message)?;
            instr_offchain.push(decompiled_instr);
        }

        // get rent for a nonce account
        let nonce_rent = client.get_minimum_balance_for_rent_exemption(State::size())?;

        // build on-chain tx
        let instr_onchain = system_instruction::create_nonce_account(
            &rent_payer_pubkey,
            &nonce_account_pubkey,
            &rent_payer_pubkey,
            nonce_rent,
        );
        let mut tx_onchain = Transaction::new_with_payer(&instr_onchain, Some(&rent_payer_pubkey));

        let signers: Vec<&dyn Signer> = vec![&nonce_account, rent_payer];
        tx_onchain.try_sign(&signers, client.get_latest_blockhash()?)?;

        client.send_and_confirm_transaction(&tx_onchain)?;

        // build off-chain tx
        instr_offchain.push(system_instruction::withdraw_nonce_account(
            &nonce_account_pubkey,
            &rent_payer_pubkey,
            &rent_payer_pubkey,
            nonce_rent,
        ));

        /*        
        instr_offchain.push(system_instruction::transfer(
            &nonce_account_pubkey,
            &rent_payer_pubkey,
            nonce_rent,
        ));
         */
        
        let message = Message::new_with_nonce(
            instr_offchain,
            Some(&rent_payer_pubkey),
            &nonce_account_pubkey,
            &rent_payer_pubkey,
        );

        let mut tx_offchain = Transaction::new_unsigned(message);

        let onchain_nonce_account = client.get_account(&nonce_account_pubkey)?;
        let nonce_hash: Hash = onchain_nonce_account.deserialize_data()?;

        let signers: Vec<&dyn Signer> = vec![rent_payer];
        tx_offchain.try_partial_sign(&signers, nonce_hash)?;
//        tx_offchain.try_partial_sign(&signers, client.get_latest_blockhash()?)?;
            
        let path = format!("{}-{}-multisig-tx", self.transaction_path.to_str().unwrap_or(""), self.transaction_name);

        write_tx_to_file(&PathBuf::from(&path), &tx_offchain)?;
        println!("the updated transaction is saved to file {}", path);

        println!("saved tx: {:#?}", tx_offchain);
        Ok(())
    }
}

impl SignTransaction {
    fn exec(&self, program_id: &Pubkey, signer: &Keypair) -> Result<()> {
        let mut tx = load_tx(&self.transaction_path)?;

        let nonce_hash = tx.message.recent_blockhash;
        let signers: Vec<&dyn Signer> = vec![signer];
        tx.try_partial_sign(&signers, nonce_hash)?;
            
        let path = format!("{}-signed", self.transaction_path.to_str().unwrap_or(""));

        write_tx_to_file(&PathBuf::from(&path), &tx)?;
        println!("the updated transaction is saved to file {}", path);

        Ok(())
    }
}

impl ExecTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Keypair) -> Result<()> {
        let mut tx = load_tx(&self.transaction_path)?;

        println!("load tx: {:#?}", tx);
        println!("signer_keys: {:#?}", tx.message.signer_keys());

        // todo: exec need to do multisig verify and pda verify


//        let nonce = fs::read(&PathBuf::from("nonce_pubkey"))?;
//        let nonce = Keypair::from_bytes(&nonce)?;
//        println!("nonce_pubkey: {:#?}", nonce.pubkey());

//        let nonce_account_before_exec = client.get_account(&nonce.pubkey())?;
//        println!("nonce_account_before_exec: {:#?}", nonce_account_before_exec);
        
//        let signers: Vec<&dyn Signer> = vec![rent_payer];
//        let nonce_hash: Hash = nonce_account_before_exec.deserialize_data()?;
//        println!("nonce hash: {:#?}", nonce_hash);

        let sig = client.send_and_confirm_transaction(&tx)?;
        println!("sig: {:#?}", sig);

//        let nonce_account_after_exec = client.get_account(&nonce.pubkey())?;
//        println!("nonce_account_after_exec: {:#?}", nonce_account_after_exec);

        Ok(())
    }
}

impl DemoTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Pubkey) -> Result<()> {
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

fn write_tx_to_file(path: &Path, tx: &Transaction) -> Result<()> {
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, tx).map_err(|e| anyhow!("{}", e))
}

fn load_tx(path: &Path) -> Result<Transaction> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let tx = serde_json::from_reader(reader)?;

    Ok(tx)
}




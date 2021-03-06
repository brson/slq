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
        MultisigTxCommand::DemoTransaction(cmd) => cmd.exec(client, program_id, &payer.pubkey()),
        _ => todo!(),
    }
}

impl StartTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Keypair) -> Result<()> {
        let nonce_account = Keypair::new();
        let nonce_account_pubkey = nonce_account.pubkey();
        let rent_payer_pubkey = rent_payer.pubkey();

        // for testing
        fs::write(&PathBuf::from("nonce_keypair"), nonce_account.to_bytes())?;
        println!("nonce_keypair saved, pubkey: {:#?}", nonce_account_pubkey);

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

        let instr_withdraw_nonce = slq::nonce::WithdrawNonceAccount::build_instruction(
            program_id,
            &rent_payer_pubkey,
            &nonce_account_pubkey,
        )?;
        instr_offchain.push(instr_withdraw_nonce);

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

        /*
                // change nonce authority to slq onchain program
                {
                    let onchain_nonce_account = client.get_account(&nonce_account_pubkey)?;
                    let nonce_data = solana_client::nonce_utils::data_from_account(&onchain_nonce_account)?;
                    println!("created nonce account before changing authority: {:#?}", onchain_nonce_account);
                    println!("nonce data: {:#?}", nonce_data);

                    let instr_change_authority = system_instruction::authorize_nonce_account(
                        &nonce_account_pubkey,
                        &rent_payer_pubkey,
                        &Pubkey::from_str("5n5nVHhyccUpChNmBw6bEFTJwLKvRMbaWHgg3WCYZfjV")?, // &program_id,
                    );
                    let mut tx_change_authority = Transaction::new_with_payer(&[instr_change_authority], Some(&rent_payer_pubkey));
                    let signers: Vec<&dyn Signer> = vec![rent_payer];
                    tx_change_authority.try_sign(&signers, client.get_latest_blockhash()?)?;
                    client.send_and_confirm_transaction(&tx_change_authority)?;

                    let onchain_nonce_account = client.get_account(&nonce_account_pubkey)?;
                    let nonce_data = solana_client::nonce_utils::data_from_account(&onchain_nonce_account)?;
                    println!("nonce account after changing authority: {:#?}", onchain_nonce_account);
                    println!("nonce data: {:#?}", nonce_data);
                }
        */
        // build off-chain tx
        let message = Message::new_with_nonce(
            instr_offchain,
            Some(&rent_payer_pubkey),
            &nonce_account_pubkey,
            &rent_payer_pubkey,
        );
        let mut tx_offchain = Transaction::new_unsigned(message);

        let onchain_nonce_account = client.get_account(&nonce_account_pubkey)?;
        let nonce_data = solana_client::nonce_utils::data_from_account(&onchain_nonce_account)?;
        let nonce_hash = nonce_data.blockhash;

        let signers: Vec<&dyn Signer> = vec![rent_payer];
        tx_offchain.try_partial_sign(&signers, nonce_hash);

        let path = format!(
            "{}-{}-start-tx",
            self.transaction_path.to_str().unwrap_or(""),
            self.transaction_name
        );

        write_tx_to_file(&PathBuf::from(&path), &tx_offchain)?;
        println!("the updated transaction is saved to file {}", path);

        println!("saved tx: {:#?}", tx_offchain);
        Ok(())
    }
}

impl SignTransaction {
    fn exec(&self, program_id: &Pubkey, signer: &Keypair) -> Result<()> {
        let mut tx = load_tx(&self.transaction_path)?;

        // for test
        let signer = fs::read(&PathBuf::from("another_signer"))?;
        let signer = Keypair::from_bytes(&signer)?;
        println!("another_signer_pubkey: {:#?}", signer.pubkey());

        let blockhash = tx.message.recent_blockhash;
        println!("message's blockhash: {:#?}", blockhash);

        let signers: Vec<&dyn Signer> = vec![&signer];
        tx.try_partial_sign(&signers, blockhash)?;

        println!("updated tx: {:#?}", tx);

        let path = format!("{}-signed", self.transaction_path.to_str().unwrap_or(""));

        write_tx_to_file(&PathBuf::from(&path), &tx)?;
        println!("the updated transaction is saved to file {}", path);

        Ok(())
    }
}

impl ExecTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Keypair) -> Result<()> {
        let tx = load_tx(&self.transaction_path)?;
        println!("exec tx: {:#?}", tx);
        // todo: exec need to do multisig verify and pda verify

        client.send_and_confirm_transaction(&tx)?;

        let nonce = fs::read(&PathBuf::from("nonce_keypair"))?;
        let nonce = Keypair::from_bytes(&nonce)?;

        // withdraw nonce
        println!("withdraw from nonce account");

        let nonce_rent = client.get_minimum_balance_for_rent_exemption(State::size())?;
        let instr_withdraw_nonce = system_instruction::withdraw_nonce_account(
            &nonce.pubkey(),
            &rent_payer.pubkey(),
            &rent_payer.pubkey(),
            nonce_rent,
        );

        let mut tx =
            Transaction::new_with_payer(&[instr_withdraw_nonce], Some(&rent_payer.pubkey()));

        let signers: Vec<&dyn Signer> = vec![rent_payer];
        tx.try_sign(&signers, client.get_latest_blockhash()?)?;

        let sig = client.send_and_confirm_transaction(&tx)?;
        println!("withdraw sig: {:#?}", sig);

        // should panic
        let nonce_account_after_withdraw = client.get_account(&nonce.pubkey())?;
        println!(
            "nonce_account_after_withdraw: {:#?}",
            nonce_account_after_withdraw
        );

        Ok(())
    }
}

impl DemoTransaction {
    fn exec(&self, client: &RpcClient, program_id: &Pubkey, rent_payer: &Pubkey) -> Result<()> {
        let another_signer = fs::read(&PathBuf::from("another_signer"))?;
        let another_signer = Keypair::from_bytes(&another_signer)?;

        let instr =
            slq::admin::ChangeApprovalThresholdAdmin::build_instruction_with_admin_accounts(
                program_id,
                rent_payer,
                &[another_signer.pubkey()],
                "foo".to_string(),
                1,
            )?;

        let tx = Transaction::new_with_payer(&[instr], Some(rent_payer));
        println!("demo tx: {:#?}", tx);

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

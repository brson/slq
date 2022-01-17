#![allow(unused)]

use anyhow::{anyhow, bail, Context, Result};
use borsh::de::BorshDeserialize;
use init::InitializeInstanceCommand;
use log::info;
use slq::init::make_instance_pda;
use slq::state::SlqInstance;
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

use admin::AdminCommand;
use multisig::MultisigCommand;
use vault::VaultCommand;

mod admin;
mod init;
mod multisig;
mod vault;

fn main() -> Result<()> {
    env_logger::init();

    let config = load_config()?;
    let client = connect(&config)?;
    let version = client.get_version()?;

    let program_keypair = get_program_keypair(&client)?;

    let opt = Opt::from_args();

    let instr = match opt.cmd {
        Command::GetInstanceState { instance_name } => {
            // todo: add program_id to config
            let (instance_pubkey, _) = make_instance_pda(&program_keypair.pubkey(), &instance_name);
            let instance_account = client.get_account(&instance_pubkey)?;
            let instance_account_data = SlqInstance::try_from_slice(&instance_account.data)?;

            println!("{:#?}", instance_account_data);

            return Ok(());
        }
        Command::InitializeInstance(cmd) => init::do_command(
            &client,
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            cmd,
        )?,
        Command::Admin(cmd) => admin::do_command(
            &client,
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            cmd,
        )?,
        Command::Multisig(cmd) => multisig::do_command(
            &client,
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            cmd,
        )?,
        Command::Vault(cmd) => vault::do_command(
            &client,
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            cmd,
        )?,
    };

    let blockhash = client.get_recent_blockhash()?.0;
    let tx = Transaction::new_signed_with_payer(
        &[instr],
        Some(&config.keypair.pubkey()),
        &[&config.keypair],
        blockhash,
    );

    let sig = client.send_and_confirm_transaction_with_spinner(&tx)?;
    info!("sig: {}", sig);

    Ok(())
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    InitializeInstance(InitializeInstanceCommand),
    GetInstanceState { instance_name: String },
    Admin(AdminCommand),
    Multisig(MultisigCommand),
    Vault(VaultCommand),
}

pub struct Config {
    json_rpc_url: String,
    keypair: Keypair,
}

fn load_config() -> Result<Config> {
    let config_file = solana_cli_config::CONFIG_FILE
        .as_ref()
        .ok_or_else(|| anyhow!("config file path"))?;
    let cli_config = solana_cli_config::Config::load(config_file)?;
    let json_rpc_url = cli_config.json_rpc_url;
    let keypair = read_keypair_file(&cli_config.keypair_path).map_err(|e| anyhow!("{}", e))?;
    Ok(Config {
        json_rpc_url,
        keypair,
    })
}

fn connect(config: &Config) -> Result<RpcClient> {
    info!("connecting to solana node at {}", config.json_rpc_url);
    let client =
        RpcClient::new_with_commitment(config.json_rpc_url.clone(), CommitmentConfig::confirmed());

    let version = client.get_version()?;
    info!("RPC version: {:?}", version);

    Ok(client)
}

static DEPLOY_PATH: &str = "target/deploy";
static PROGRAM_KEYPAIR_PATH: &str = "slq-keypair.json";

pub fn get_program_keypair(client: &RpcClient) -> Result<Keypair> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let deploy_path = format!("{}/../../{}", manifest_dir, DEPLOY_PATH);
    let program_keypair_path = format!("{}/{}", deploy_path, PROGRAM_KEYPAIR_PATH);

    info!("loading program keypair from {}", program_keypair_path);

    let program_keypair = read_keypair_file(&program_keypair_path)
        .map_err(|e| anyhow!("{}", e))
        .context("unable to load program keypair")?;

    let program_id = program_keypair.pubkey();

    info!("program id: {}", program_id);

    let account = client
        .get_account(&program_id)
        .context("unable to get program account")?;

    info!("program account: {:?}", account);

    if !account.executable {
        bail!("solana account not executable");
    }

    Ok(program_keypair)
}

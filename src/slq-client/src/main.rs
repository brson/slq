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

pub struct Config {
    json_rpc_url: String,
    keypair: Keypair,
}

fn load_config() -> Result<Config> {
    let config_file = solana_cli_config::CONFIG_FILE
        .as_ref()
        .ok_or_else(|| anyhow!("config file path"))?;
    let cli_config = solana_cli_config::Config::load(&config_file)?;
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

fn main() -> Result<()> {
    env_logger::init();

    let config = load_config()?;
    let client = connect(&config)?;
    let version = client.get_version()?;

    let program_keypair = get_program_keypair(&client)?;
    let vault_name = "vault".to_string();

    let opt = Opt::from_args();

    let instr = match opt.cmd {
        Command::Admin(cmd) => do_admin_command(
            &client,
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            cmd,
        )?,
        Command::CreateVault => slq::CreateVault::build_instruction(
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            &vault_name,
        )?,
        Command::DepositToVault { amount } => slq::DepositToVault::build_instruction(
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            &vault_name,
            amount,
        )?,
        Command::WithdrawFromVault { amount } => slq::WithdrawFromVault::build_instruction(
            &program_keypair.pubkey(),
            &config.keypair.pubkey(),
            &vault_name,
            amount,
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

fn do_admin_command(
    client: &RpcClient,
    program_id: &Pubkey,
    rent_payer: &Pubkey,
    cmd: AdminCommand,
) -> Result<Instruction> {
    use slq::admin;

    match cmd {
        AdminCommand::Init(InitAdminCommand {
            instance_name,
            approval_threshold,
            admin_accounts,
        }) => {
            // todo space for admin storage
            let storage: u64 = 1024;
            //  todo solana_sdk::borsh::get_instance_packed_len
            let lamports =
                client.get_minimum_balance_for_rent_exemption(storage.try_into()?)?;

            let admin_accounts = admin_accounts
                .iter()
                .map(|account| Pubkey::from_str(account))
                .collect::<Result<Vec<Pubkey>, _>>()?;

            admin::Init::build_instruction(
                program_id,
                rent_payer,
                lamports,
                storage,
                instance_name,
                approval_threshold,
                admin_accounts,
            )
        }
        _ => todo!(),
    }
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    Admin(AdminCommand),
    CreateVault,
    DepositToVault { amount: u64 },
    WithdrawFromVault { amount: u64 }, // todo: withdraw all command
}

#[derive(StructOpt, Debug)]
enum AdminCommand {
    Init(InitAdminCommand),
    ChangeApprovalThreshold(ChangeApprovalThresholdAdminCommand),
    AddAdminAccount(AddAdminAccountAdminCommand),
    RemoveAdminAccount(RemoveAdminAccountAdminCommand),
}

#[derive(StructOpt, Debug)]
struct InitAdminCommand {
    instance_name: String,
    approval_threshold: u8,
    admin_accounts: Vec<String>,
}

#[derive(StructOpt, Debug)]
struct ChangeApprovalThresholdAdminCommand {
    instance_name: String,
    approval_threshold: u8,
}

#[derive(StructOpt, Debug)]
struct AddAdminAccountAdminCommand {
    instance_name: String,
    account: String,
}

#[derive(StructOpt, Debug)]
struct RemoveAdminAccountAdminCommand {
    instance_name: String,
    account: String,
}

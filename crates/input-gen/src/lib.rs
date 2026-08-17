use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;

mod client;
mod common;
mod processor;
mod provider;

use client::{create_client, Client};
use provider::{eest::EestProvider, rpc::RpcProvider, InputProvider};

#[derive(Args, Debug, Clone)]
pub struct InputGenArgs {
    /// Execution client to generate inputs for
    #[arg(short, long, value_enum, default_value = "reth")]
    client: Client,

    /// Provider of inputs
    #[command(subcommand)]
    provider: ProviderCommand,

    /// Output folder for the generated ZisK input files (default: <client>-inputs)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand, Clone, Debug)]
enum ProviderCommand {
    /// Generate from EEST fixtures
    Eest(#[command(flatten)] EestProvider),
    /// Generate from RPC endpoint
    Rpc(#[command(flatten)] RpcProvider),
}

pub async fn run(args: InputGenArgs) -> Result<()> {
    let client = create_client(args.client);

    let output = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}-inputs", client.name())));

    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output folder: {}", output.display()))?;

    match args.provider {
        ProviderCommand::Eest(eest_provider) => {
            eest_provider
                .generate_inputs(client.as_ref(), &output)
                .await?;
        }
        ProviderCommand::Rpc(rpc_provider) => {
            rpc_provider
                .generate_inputs(client.as_ref(), &output)
                .await?;
        }
    }

    Ok(())
}

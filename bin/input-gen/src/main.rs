use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod client;
mod common;
mod processor;
mod provider;

use client::{create_client, Client};
use provider::{eest::EestProvider, rpc::RpcProvider, InputProvider};

#[derive(Parser)]
#[command(version, about = "Generate ZisK inputs from a variety of providers for different execution clients", long_about = None)]
struct Cli {
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Create execution client
    let client = create_client(cli.client);

    // Define output directory
    let output = cli
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}-inputs", client.name())));

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output folder: {}", output.display()))?;

    match cli.provider {
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

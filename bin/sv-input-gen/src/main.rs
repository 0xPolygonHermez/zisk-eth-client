// TODO: Integrate the fixtures-witness relationship via the witness or witness-cli crate
// TODO: Add old blocks via local reth node

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod fixtures;
mod rpc;

use fixtures::process_fixtures;
use rpc::process_rpc;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum OutputFormat {
    /// Binary format
    #[default]
    Binary,
    /// JSON format
    Json,
}

#[derive(Parser)]
#[command(name = "reth-input-generator")]
#[command(about = "Generate Reth zkVM inputs from StatelessValidationFixture files")]
#[command(version)]
struct Cli {
    /// Source of inputs
    #[command(subcommand)]
    source: SourceCommand,

    /// Output folder for generated Reth input files
    #[arg(short, long, default_value = "reth-inputs")]
    output: PathBuf,

    /// Output format
    #[arg(short, long, default_value = "binary")]
    format: OutputFormat,
}

#[derive(Subcommand, Clone, Debug)]
enum SourceCommand {
    /// Generate inputs from StatelessValidationFixture JSON files
    Fixtures {
        /// Input folder containing StatelessValidationFixture JSON files
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Generate inputs from an RPC endpoint
    Rpc {
        /// RPC URL to use (mandatory)
        #[arg(long)]
        rpc_url: String,

        /// Specific block number to fetch
        #[arg(long, conflicts_with = "last_n_blocks")]
        block: Option<u64>,

        /// Number of last blocks to fetch
        #[arg(long, conflicts_with = "block")]
        last_n_blocks: Option<usize>,

        /// Optional RPC headers (format: "Key:Value")
        #[arg(long)]
        rpc_header: Option<Vec<String>>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&cli.output)
        .with_context(|| format!("Failed to create output folder: {}", cli.output.display()))?;

    match cli.source {
        SourceCommand::Fixtures { input } => {
            process_fixtures(&input, &cli.output, cli.format)?;
        }
        SourceCommand::Rpc {
            rpc_url,
            block,
            last_n_blocks,
            rpc_header,
        } => {
            process_rpc(
                &rpc_url,
                block,
                last_n_blocks,
                rpc_header,
                &cli.output,
                cli.format,
            )
            .await?;
        }
    }

    Ok(())
}

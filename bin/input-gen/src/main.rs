mod common;
mod rpc;
mod tests;
mod types;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use rpc::zisk_inputs_from_rpc;
use tests::zisk_inputs_from_eest;
use types::{ExecutionClient, OutputFormat};

#[derive(Parser)]
#[command(name = "input-generator")]
#[command(about = "Generate ZisK inputs from a variety of sources")]
#[command(version)]
struct Cli {
    /// Output format
    #[arg(short, long, default_value = "binary")]
    format: OutputFormat,

    /// Output folder for the generated ZisK input files (default: <client>-inputs)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Execution client to generate inputs for
    #[arg(short, long, value_enum, default_value = "reth")]
    client: ExecutionClient,

    /// Source of inputs
    #[command(subcommand)]
    source: SourceCommand,
}

#[derive(Subcommand, Clone, Debug)]
enum SourceCommand {
    /// Generate inputs from execution specification tests (EEST)
    Tests {
        /// EEST release tag to use (e.g., "v0.1.0"). If empty, the latest release will be used.
        #[arg(short, long, conflicts_with = "eest_fixtures_path")]
        tag: Option<String>,

        /// Input folder for EEST files. If not provided, --tag is required.
        #[arg(long, conflicts_with = "tag")]
        eest_fixtures_path: Option<PathBuf>,

        /// Include only test names containing the provided strings.
        #[arg(short, long)]
        include: Option<Vec<String>>,

        /// Exclude all test names containing the provided strings.
        #[arg(short, long)]
        exclude: Option<Vec<String>>,

        /// Number of threads for parallel processing (default: all available)
        #[arg(long, default_value = "10")]
        threads: Option<usize>,
    },

    /// Generate inputs from an RPC endpoint
    Rpc {
        /// RPC URL to use (mandatory)
        #[arg(short = 'u', long)]
        rpc_url: String,

        /// Optional RPC headers (format: "Key:Value")
        #[arg(short = 'h', long)]
        rpc_headers: Option<Vec<String>>,

        /// Specific block number to fetch
        #[arg(long, conflicts_with_all = ["last_n_blocks", "follow"])]
        block: Option<u64>,

        /// Number of last blocks to fetch
        #[arg(long, conflicts_with_all = ["block", "follow"])]
        last_n_blocks: Option<usize>,

        /// Listen for new blocks
        #[arg(long, default_value_t = false, conflicts_with_all = ["last_n_blocks", "block"])]
        follow: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let output = cli
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}-inputs", cli.client)));

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output folder: {}", output.display()))?;

    match cli.source {
        SourceCommand::Tests {
            tag,
            include,
            exclude,
            eest_fixtures_path,
            threads,
        } => {
            zisk_inputs_from_eest(
                tag,
                include,
                exclude,
                eest_fixtures_path,
                &output,
                cli.format,
                threads,
                &cli.client,
            )
            .await?;
        }

        SourceCommand::Rpc {
            rpc_url,
            rpc_headers,
            block,
            last_n_blocks,
            follow,
        } => {
            zisk_inputs_from_rpc(
                &rpc_url,
                rpc_headers,
                block,
                last_n_blocks,
                follow,
                &output,
                cli.format,
                &cli.client,
            )
            .await?;
        }
    }

    Ok(())
}

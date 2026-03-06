mod common;
mod rpc;
mod tests;
mod types;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use rpc::reth_input_files_from_rpc;
use tests::reth_input_files_from_eest;
use types::OutputFormat;

#[derive(Parser)]
#[command(name = "reth-input-generator")]
#[command(about = "Generate Reth zkVM inputs from StatelessValidationFixture files")]
#[command(version)]
struct Cli {
    /// Output format
    #[arg(short, long, default_value = "binary")]
    format: OutputFormat,

    /// Output folder for generated Reth input files
    #[arg(short, long, default_value = "reth-inputs")]
    output: PathBuf,

    /// Source of inputs
    #[command(subcommand)]
    source: SourceCommand,
}

#[derive(Subcommand, Clone, Debug)]
enum SourceCommand {
    /// Generate inputs from Ethereum Execution Specification Tests (EEST)
    Eest {
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
        #[arg(long, group = "block_selection")]
        block: Option<u64>,

        /// Number of last blocks to fetch
        #[arg(long, group = "block_selection")]
        last_n_blocks: Option<usize>,

        /// Fetch blocks in a range (inclusive)
        #[arg(long, num_args = 2, value_names = ["START", "END"], group = "block_selection")]
        range_of_blocks: Option<Vec<u64>>,

        /// Listen for new blocks
        #[arg(long, default_value_t = false, group = "block_selection")]
        follow: bool,
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
        SourceCommand::Eest {
            tag,
            include,
            exclude,
            eest_fixtures_path,
            threads,
        } => {
            reth_input_files_from_eest(
                tag,
                include,
                exclude,
                eest_fixtures_path,
                &cli.output,
                cli.format,
                threads,
            )
            .await?;
        }

        SourceCommand::Rpc {
            rpc_url,
            rpc_headers,
            block,
            last_n_blocks,
            range_of_blocks,
            follow,
        } => {
            reth_input_files_from_rpc(
                &rpc_url,
                rpc_headers,
                block,
                last_n_blocks,
                range_of_blocks,
                follow,
                &cli.output,
                cli.format,
            )
            .await?;
        }
    }

    Ok(())
}

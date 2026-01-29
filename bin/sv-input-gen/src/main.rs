// TODO: Integrate the fixtures-witness relationship via the witness or witness-cli crate

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use jsonrpsee::http_client::{HeaderMap, HttpClientBuilder};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use walkdir::WalkDir;

use alloy_eips::BlockNumberOrTag;
use alloy_genesis::ChainConfig;
use alloy_rpc_types_eth::{Block, Header, Receipt, Transaction, TransactionRequest};

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::TransactionSigned;
use reth_rpc_api::{DebugApiClient, EthApiClient};
use reth_stateless::StatelessInput;

use stateless_validator_reth::guest::StatelessValidatorRethInput;

/// A stateless validation fixture containing block data and witness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidationFixture {
    /// Name of the blockchain test case (e.g., "`ModExpAttackContract`").
    pub name: String,
    /// The stateless input for the block validation.
    pub stateless_input: StatelessInput,
    /// Whether the stateless block validation is successful.
    pub success: bool,
}

/// Reads all StatelessValidationFixture files from a directory.
pub fn read_fixtures(path: &Path) -> Result<Vec<StatelessValidationFixture>> {
    WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
        })
        .par_bridge()
        .map(|entry| {
            let content = std::fs::read(entry.path())?;
            let fixture: StatelessValidationFixture = serde_json::from_slice(&content)
                .with_context(|| format!("Failed to parse {}", entry.path().display()))?;
            Ok(fixture)
        })
        .collect()
}

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

    info!("Writing inputs to: {}", cli.output.display());

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

fn process_fixtures(input: &Path, output: &Path, format: OutputFormat) -> Result<()> {
    info!("Reading fixtures from: {}", input.display());

    let fixtures = read_fixtures(input)?;
    info!("Found {} fixtures", fixtures.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for fixture in &fixtures {
        match generate_reth_inputs_from_fixtures(fixture, output, format) {
            Ok(_) => {
                info!("Generated input for: {}", fixture.name);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate input for {}: {}", fixture.name, e);
                error_count += 1;
            }
        }
    }

    info!(
        "Completed: {} succeeded, {} failed",
        success_count, error_count
    );

    Ok(())
}

async fn process_rpc(
    rpc_url: &str,
    block: Option<u64>,
    last_n_blocks: Option<usize>,
    rpc_headers: Option<Vec<String>>,
    output: &Path,
    format: OutputFormat,
) -> Result<()> {
    info!("Connecting to RPC: {}", rpc_url);

    // Build headers if provided
    let mut header_map = HeaderMap::new();
    if let Some(headers) = rpc_headers {
        for header in headers {
            let (key, value) = header
                .split_once(':')
                .with_context(|| format!("Invalid header format: {}", header))?;
            header_map.insert(
                key.trim().parse::<http::HeaderName>()?,
                value.trim().parse::<http::HeaderValue>()?,
            );
        }
    }

    // Build HTTP client
    let client = HttpClientBuilder::default()
        .set_headers(header_map)
        .max_response_size(1 << 30)
        .build(rpc_url)
        .with_context(|| "Failed to build HTTP client")?;

    // Fetch chain ID and determine chain config
    let chain_id = EthApiClient::<(), (), (), (), (), ()>::chain_id(&client)
        .await
        .with_context(|| "Failed to fetch chain ID")?
        .with_context(|| "Chain ID not found")?;

    let chain = Chain::from_id(chain_id.to());
    let chain_config = match chain.named() {
        Some(NamedChain::Mainnet) => mainnet_chain_config(),
        Some(NamedChain::Sepolia) => SEPOLIA.genesis.config.clone(),
        Some(NamedChain::Hoodi) => HOODI.genesis.config.clone(),
        Some(NamedChain::Holesky) => HOLESKY.genesis.config.clone(),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    info!("Connected to chain: {:?} (ID: {})", chain.named(), chain_id);

    // Determine which blocks to fetch
    let block_numbers: Vec<u64> = if let Some(block_num) = block {
        vec![block_num]
    } else {
        let n = last_n_blocks.unwrap_or(1);
        let latest = fetch_latest_block_number(&client).await?;
        let start = latest.saturating_sub(n as u64 - 1);
        (start..=latest).collect()
    };

    info!("Fetching {} block(s)...", block_numbers.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for block_num in block_numbers {
        match fetch_and_generate_input(&client, block_num, &chain_config, output, format).await {
            Ok(_) => {
                info!("Generated input for block: {}", block_num);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate input for block {}: {}", block_num, e);
                error_count += 1;
            }
        }
    }

    info!(
        "Completed: {} succeeded, {} failed",
        success_count, error_count
    );

    Ok(())
}

async fn fetch_latest_block_number(client: &jsonrpsee::http_client::HttpClient) -> Result<u64> {
    let block = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block,
        Receipt,
        Header,
        TransactionSigned,
    >::block_by_number(client, BlockNumberOrTag::Latest, false)
    .await
    .with_context(|| "Failed to fetch latest block")?
    .with_context(|| "Latest block not found")?;

    Ok(block.header.number)
}

async fn fetch_and_generate_input(
    client: &jsonrpsee::http_client::HttpClient,
    block_num: u64,
    chain_config: &ChainConfig,
    output: &Path,
    format: OutputFormat,
) -> Result<()> {
    // Fetch the execution witness
    let witness =
        DebugApiClient::<()>::debug_execution_witness(client, BlockNumberOrTag::Number(block_num))
            .await
            .with_context(|| {
                format!("Failed to fetch execution witness for block {}", block_num)
            })?;

    // Fetch the block
    let block = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block<TransactionSigned>,
        Receipt,
        Header,
        TransactionSigned,
    >::block_by_number(client, BlockNumberOrTag::Number(block_num), true)
    .await
    .with_context(|| format!("Failed to fetch block {}", block_num))?
    .with_context(|| format!("Block {} not found", block_num))?;

    // Create the fixture
    let fixture = StatelessValidationFixture {
        name: format!("rpc_block_{}", block_num),
        stateless_input: StatelessInput {
            block: block.into_consensus(),
            witness,
            chain_config: chain_config.clone(),
        },
        success: true,
    };

    // Generate input
    generate_reth_inputs_from_fixtures(&fixture, output, format)
}

fn generate_reth_inputs_from_fixtures(
    fixture: &StatelessValidationFixture,
    output_dir: &Path,
    format: OutputFormat,
) -> Result<()> {
    let reth_input = StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
        .with_context(|| {
            format!(
                "Failed to create StatelessValidatorReth input for {}",
                fixture.name
            )
        })?;

    let filename = sanitize_filename(&fixture.name);

    match format {
        OutputFormat::Binary => {
            let bin_dir = output_dir.join("bin");
            std::fs::create_dir_all(&bin_dir)?;
            let output_path = bin_dir.join(format!("{}.bin", filename));
            let bytes = bincode::serialize(&reth_input)?;
            std::fs::write(&output_path, bytes)?;
        }
        OutputFormat::Json => {
            let json_dir = output_dir.join("json");
            std::fs::create_dir_all(&json_dir)?;
            let output_path = json_dir.join(format!("{}.json", filename));
            let json = serde_json::to_string_pretty(&reth_input)?;
            std::fs::write(&output_path, json)?;
        }
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

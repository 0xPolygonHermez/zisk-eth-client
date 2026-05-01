use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

use ethrex_common::types::block_execution_witness::RpcExecutionWitness;
use ethrex_common::types::{Block, ChainConfig};
use ethrex_config::networks::Network;
use ethrex_rpc::debug::execution_witness::execution_witness_from_rpc_chain_config;
use ethrex_rpc::types::{block::RpcBlock, block_identifier::BlockIdentifier};
use ethrex_rpc::EthClient;

use guest_ethrex::{get_chain_name, EthrexInput};
use zisk_sdk::ZiskStdin;

#[derive(Parser)]
#[command(name = "zisk-ethrex-input-generator")]
#[command(about = "Generate ZisK inputs for the ethrex client from RPC")]
#[command(version)]
struct Cli {
    /// RPC URL to use
    #[arg(short = 'u', long)]
    rpc_url: String,

    /// Output folder for the generated ZisK input files (default: ethrex-inputs)
    #[arg(short, long, default_value = "ethrex-inputs")]
    output: PathBuf,

    /// Block selection method (default: last block)
    #[command(subcommand)]
    command: Option<BlockSelection>,
}

#[derive(Subcommand, Clone, Debug)]
enum BlockSelection {
    /// Number of last blocks to fetch (default: 1 if no other block selection method is used)
    Last {
        /// Number of blocks to fetch (default: 1)
        #[arg(short, long, default_value = "1")]
        count: usize,
    },
    /// Specific block number to fetch
    Block {
        /// Block number to fetch
        #[arg(short, long)]
        number: u64,
    },
    /// Fetch blocks in a range (inclusive)
    Range {
        /// Start block number
        #[arg(short, long)]
        start: u64,
        /// End block number
        #[arg(short, long)]
        end: u64,
    },
    /// Follow new blocks continuously
    Follow,
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

    // Create EthClient
    let url = Url::parse(&cli.rpc_url)?;
    let rpc_client = EthClient::new(url)?;

    // Fetch chain ID to determine network
    let chain_id = rpc_client.get_chain_id().await?.as_u64();
    let chain_name = get_chain_name(chain_id);

    info!("Connected to chain: {} (ID: {})", chain_name, chain_id);

    info!("Generating inputs for the Ethrex client...");

    // Get chain config
    let chain_config = Network::try_from(chain_id)
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .get_genesis()?
        .config;

    // Determine block selection method. It defaults to fetching the last block if no other method is specified.
    let command = cli.command.unwrap_or(BlockSelection::Last { count: 1 });
    match command {
        BlockSelection::Last { count } => {
            if count == 0 {
                info!("No blocks to process (count = 0)");
                return Ok(());
            }
            let latest = fetch_latest_block_number(&rpc_client).await?;
            let start = latest.saturating_sub(count as u64 - 1);
            let block_numbers: Vec<u64> = (start..=latest).collect();
            process_blocks(
                &rpc_client,
                &chain_config,
                chain_name,
                &cli.output,
                block_numbers,
            )
            .await?;
        }
        BlockSelection::Block { number } => {
            process_blocks(
                &rpc_client,
                &chain_config,
                chain_name,
                &cli.output,
                vec![number],
            )
            .await?;
        }
        BlockSelection::Range { start, end } => {
            anyhow::ensure!(start <= end, "START ({start}) must be <= END ({end})");
            let block_numbers: Vec<u64> = (start..=end).collect();
            process_blocks(
                &rpc_client,
                &chain_config,
                chain_name,
                &cli.output,
                block_numbers,
            )
            .await?;
        }
        BlockSelection::Follow => {
            follow_new_blocks(&rpc_client, &chain_config, chain_name, &cli.output).await?;
        }
    }

    Ok(())
}

/// Fetch the latest block number
async fn fetch_latest_block_number(client: &EthClient) -> Result<u64> {
    let block_number = client.get_block_number().await?;
    Ok(block_number.as_u64())
}

/// Fetch block and execution witness for a given block number
async fn fetch_block_and_witness(
    client: &EthClient,
    block_num: u64,
) -> Result<(RpcBlock, RpcExecutionWitness)> {
    // Fetch block
    let block: RpcBlock = client
        .get_block_by_number(BlockIdentifier::Number(block_num), true)
        .await?;

    // Fetch execution witness
    let witness: RpcExecutionWitness = client
        .get_witness(BlockIdentifier::Number(block_num), None)
        .await?;

    Ok((block, witness))
}

/// Process a batch of blocks
async fn process_blocks(
    client: &EthClient,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    block_numbers: Vec<u64>,
) -> Result<()> {
    info!(
        "Processing {} block(s): {:?}",
        block_numbers.len(),
        block_numbers
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for block_num in block_numbers {
        match process_single_block(client, chain_config, chain_name, output, block_num).await {
            Ok(_) => {
                info!("Successfully processed Block #{}", block_num);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to process Block #{}: {:?}", block_num, e);
                error_count += 1;
            }
        }
    }

    info!(
        "Processing complete: {} successful, {} failed",
        success_count, error_count
    );

    Ok(())
}

/// Process a single block
async fn process_single_block(
    client: &EthClient,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    block_num: u64,
) -> Result<()> {
    // Fetch block and witness
    let (rpc_block, rpc_witness) = fetch_block_and_witness(client, block_num).await?;

    // Convert RPC types to internal types
    let block: Block = rpc_block
        .try_into()
        .map_err(|e: String| anyhow::anyhow!("{}", e))?;

    let witness = execution_witness_from_rpc_chain_config(rpc_witness, *chain_config, block_num)
        .context("Failed to convert execution witness")?;

    // Create EthrexInput
    let input = EthrexInput::new(block.clone(), witness);

    // Serialize to ZiskStdin
    let zisk_stdin = ZiskStdin::new();

    let input_bytes = input.serialize()?;
    zisk_stdin.write_slice(&input_bytes);

    // Generate filename
    let tx_count = block.body.transactions.len();
    let gas_used = block.header.gas_used;
    let mgas = gas_used / 1_000_000;

    let filename = format!(
        "{}_{}_{}_{}_zec_ethrex.bin",
        chain_name.to_lowercase(),
        block_num,
        tx_count,
        mgas
    );

    let output_path = output.join(&filename);

    // Write to file
    zisk_stdin.save(&output_path)?;

    Ok(())
}

/// Continuously follow and process new blocks
async fn follow_new_blocks(
    rpc_client: &EthClient,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
) -> Result<()> {
    info!("Following new blocks (press Ctrl+C to stop)...");

    let stop = CancellationToken::new();
    let stop_clone = stop.clone();

    // Spawn a task to handle Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        info!("Received Ctrl+C, stopping...");
        stop_clone.cancel();
    });

    let mut success_count = 0;
    let mut error_count = 0;
    let mut next_block_num = fetch_latest_block_number(rpc_client).await?;

    loop {
        if stop.is_cancelled() {
            break;
        }

        let latest = fetch_latest_block_number(rpc_client).await?;

        for block_num in next_block_num..=latest {
            if stop.is_cancelled() {
                break;
            }

            match process_single_block(rpc_client, chain_config, chain_name, output, block_num)
                .await
            {
                Ok(_) => {
                    info!("Successfully processed Block #{}", block_num);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to process block #{}: {:?}", block_num, e);
                    error_count += 1;
                }
            }
        }

        next_block_num = latest + 1;

        // Wait before polling again (average block time is ~12s)
        tokio::select! {
            _ = stop.cancelled() => {
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(6)) => {}
        }
    }

    info!(
        "Follow complete: {} successful, {} failed",
        success_count, error_count
    );

    Ok(())
}

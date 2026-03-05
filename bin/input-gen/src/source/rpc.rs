// TODO: Add old blocks via local reth node
// TODO: Simplify when the `debug_execution_witness_by_block_hash` method gets available

use alloy_eips::BlockNumberOrTag;
use alloy_genesis::ChainConfig;
use alloy_rpc_types_eth::{Block, Header, Receipt, Transaction, TransactionRequest};
use anyhow::{Context, Result};
use jsonrpsee::http_client::{HeaderMap, HttpClient, HttpClientBuilder};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::TransactionSigned;
use reth_rpc_api::{DebugApiClient, EthApiClient};

use stateless_reth::StatelessInput;

use witness_generator::StatelessValidationFixture;

use crate::client::ExecutionClient;

#[allow(clippy::too_many_arguments)]
pub async fn zisk_inputs_from_rpc(
    rpc_url: &str,
    rpc_headers: Option<Vec<String>>,
    block: Option<u64>,
    last_n_blocks: Option<usize>,
    range_of_blocks: Option<Vec<u64>>,
    follow: bool,
    output: &Path,
    client: &dyn ExecutionClient,
) -> Result<()> {
    info!("Connecting to RPC: {}", rpc_url);

    // Initialize RPC client
    let (rpc_client, chain_config, chain_name) = init_rpc_client(rpc_url, rpc_headers).await?;

    info!(
        "Connected to chain: {} (ID: {})",
        chain_name, chain_config.chain_id
    );

    // If follow is enabled, continuously listen for new blocks.
    if follow {
        return follow_new_blocks(&rpc_client, &chain_config, chain_name, output, client).await;
    }

    // Otherwise, process specified blocks.
    let block_numbers: Vec<u64> = if let Some(block_num) = block {
        // Single block
        vec![block_num]
    } else if let Some(range) = range_of_blocks {
        // Range of blocks
        if range.len() != 2 {
            anyhow::bail!("Range requires exactly 2 values: START and END");
        }
        let (start, end) = (range[0], range[1]);
        if start > end {
            anyhow::bail!("Range START ({}) must be <= END ({})", start, end);
        }
        (start..=end).collect()
    } else {
        // Last N blocks (default: 1)
        let n = last_n_blocks.unwrap_or(1);
        if n == 0 {
            info!("No blocks to process (last_n_blocks = 0)");
            return Ok(());
        }
        let latest = fetch_latest_block_number(&rpc_client).await?;
        let start = latest.saturating_sub(n as u64 - 1);
        (start..=latest).collect()
    };

    info!(
        "Processing {} block(s): {:?}",
        block_numbers.len(),
        block_numbers
    );

    let mut success_count = 0;
    let mut error_count = 0;
    for block_num in block_numbers {
        match process_block_for_client(
            &rpc_client,
            block_num,
            &chain_config,
            chain_name,
            output,
            client,
        )
        .await
        {
            Ok(_) => {
                info!("Generated input for block: {}", block_num);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate input for block {}: {:?}", block_num, e);
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

/// Process a single block and generate input for the specified client
async fn process_block_for_client(
    rpc_client: &HttpClient,
    block_num: u64,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    client: &dyn ExecutionClient,
) -> Result<()> {
    let (fixture, fixture_name) = fetch_fixture(
        rpc_client,
        block_num,
        chain_config,
        chain_name,
        client.name(),
    )
    .await?;

    let result = client.generate_input(&fixture)?;
    result.save_to_file(&fixture_name, output)?;

    Ok(())
}

async fn init_rpc_client(
    rpc_url: &str,
    rpc_headers: Option<Vec<String>>,
) -> Result<(
    jsonrpsee::http_client::HttpClient,
    ChainConfig,
    &'static str,
)> {
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
    let rpc_client = HttpClientBuilder::default()
        .set_headers(header_map)
        .max_response_size(1 << 30)
        .build(rpc_url)
        .context("Failed to build HTTP client")?;

    // Fetch chain ID and determine chain config
    let chain_id = EthApiClient::<(), (), (), (), (), ()>::chain_id(&rpc_client)
        .await
        .context("Failed to fetch chain ID")?
        .context("Chain ID not found")?;

    let chain = Chain::from_id(chain_id.to());
    let (chain_config, chain_name) = match chain.named() {
        Some(NamedChain::Mainnet) => (mainnet_chain_config(), "mainnet"),
        Some(NamedChain::Sepolia) => (SEPOLIA.genesis.config.clone(), "sepolia"),
        Some(NamedChain::Hoodi) => (HOODI.genesis.config.clone(), "hoodi"),
        Some(NamedChain::Holesky) => (HOLESKY.genesis.config.clone(), "holesky"),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    Ok((rpc_client, chain_config, chain_name))
}

/// Continuously follow and process new blocks
async fn follow_new_blocks(
    rpc_client: &HttpClient,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    client: &dyn ExecutionClient,
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

    let mut next_block_num = fetch_latest_block_number(rpc_client).await?;
    let mut success_count = 0;
    let mut error_count = 0;

    loop {
        tokio::select! {
            _ = stop.cancelled() => {
                info!("Stopped following blocks.");
                break;
            }
            result = process_new_blocks(rpc_client, &mut next_block_num, chain_config, chain_name, output, client) => {
                match result {
                    Ok((successes, errors)) => {
                        success_count += successes;
                        error_count += errors;
                    }
                    Err(e) => {
                        warn!("Error processing blocks: {:?}", e);
                    }
                }
            }
        }

        // Wait before polling again (average block time is ~12s)
        tokio::select! {
            _ = stop.cancelled() => {
                info!("Stopped following blocks.");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(6)) => {}
        }
    }

    info!(
        "Follow mode completed: {} succeeded, {} failed",
        success_count, error_count
    );

    Ok(())
}

/// Process any new blocks from next_block_num to latest
async fn process_new_blocks(
    rpc_client: &HttpClient,
    next_block_num: &mut u64,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    client: &dyn ExecutionClient,
) -> Result<(usize, usize)> {
    let latest = fetch_latest_block_number(rpc_client).await?;

    if *next_block_num > latest {
        return Ok((0, 0));
    }

    let mut success_count = 0;
    let mut error_count = 0;
    for block_num in *next_block_num..=latest {
        match process_block_for_client(
            rpc_client,
            block_num,
            chain_config,
            chain_name,
            output,
            client,
        )
        .await
        {
            Ok(_) => {
                info!("Generated input for block: {}", block_num);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate input for block {}: {:?}", block_num, e);
                error_count += 1;
            }
        }
    }

    *next_block_num = latest + 1;

    Ok((success_count, error_count))
}

async fn fetch_latest_block_number(rpc_client: &HttpClient) -> Result<u64> {
    let block = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block,
        Receipt,
        Header,
        TransactionSigned,
    >::block_by_number(rpc_client, BlockNumberOrTag::Latest, false)
    .await
    .context("Failed to fetch latest block")?
    .context("Latest block not found")?;

    Ok(block.header.number)
}

/// Fetch block data and create a fixture for the specified client
async fn fetch_fixture(
    rpc_client: &HttpClient,
    block_num: u64,
    chain_config: &ChainConfig,
    chain_name: &str,
    client_name: &str,
) -> Result<(StatelessValidationFixture, String)> {
    // Fetch the execution witness using debug_execution_witness
    let witness = DebugApiClient::<()>::debug_execution_witness(
        rpc_client,
        BlockNumberOrTag::Number(block_num),
    )
    .await
    .with_context(|| format!("Failed to fetch execution witness for block {}", block_num))?;

    // Fetch the block
    let block = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block<TransactionSigned>,
        Receipt,
        Header,
        TransactionSigned,
    >::block_by_number(rpc_client, BlockNumberOrTag::Number(block_num), true)
    .await
    .with_context(|| format!("Failed to fetch block {}", block_num))?
    .with_context(|| format!("Block {} not found", block_num))?;

    // Get transaction count and gas used from the block
    let tx_count = block.transactions.len();
    let gas_used = block.header.gas_used;
    let mgas = gas_used / 1_000_000;

    // Create the fixture
    let fixture_name = format!(
        "{}_{}_{}_{}_zec_{}",
        chain_name, block_num, tx_count, mgas, client_name
    );
    let fixture = StatelessValidationFixture {
        name: fixture_name.clone(),
        stateless_input: StatelessInput {
            block: block.into_consensus(),
            witness,
            chain_config: chain_config.clone(),
        },
        success: true,
    };

    Ok((fixture, fixture_name))
}

// TODO: Add old blocks via archive reth node
// TODO: Simplify when the `debug_execution_witness_by_block_hash` method gets available

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
use stateless_reth::{ExecutionWitness, StatelessInput};

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
    // Initialize the RPC client
    let (rpc_client, chain_config, chain_name) = init_rpc_client(rpc_url, rpc_headers).await?;

    info!(
        "Connected to chain: {} (ID: {})",
        chain_name, chain_config.chain_id
    );

    let client_name = client.display_name();
    info!("Generating inputs for the {} client...", client_name);

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
    } else  {
        // Default to last N blocks (default N=1)
        let n = last_n_blocks.unwrap_or(1);

        // Last N blocks
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
        match process_block(
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
                info!("Generated {} input for block: {}", client_name, block_num);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate {} input for block {}: {:?}", client_name, block_num, e);
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
    let client = HttpClientBuilder::default()
        .set_headers(header_map)
        .max_response_size(1 << 30)
        .build(rpc_url)
        .context("Failed to build HTTP client")?;

    // Fetch chain ID and determine chain config
    let chain_id = EthApiClient::<(), (), (), (), (), ()>::chain_id(&client)
        .await
        .context("Failed to fetch chain ID")?
        .context("Chain ID not found")?;

    let chain = Chain::from_id(chain_id.to());

    let (chain_config, chain_name) = match chain.named() {
        Some(NamedChain::Mainnet) => (mainnet_chain_config(), "Mainnet"),
        Some(NamedChain::Sepolia) => (SEPOLIA.genesis.config.clone(), "Sepolia"),
        Some(NamedChain::Hoodi) => (HOODI.genesis.config.clone(), "Hoodi"),
        Some(NamedChain::Holesky) => (HOLESKY.genesis.config.clone(), "Holesky"),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    Ok((client, chain_config, chain_name))
}

/// Process a single block and generate input for the specified client
async fn process_block(
    rpc_client: &HttpClient,
    block_num: u64,
    chain_config: &ChainConfig,
    chain_name: &str,
    output: &Path,
    client: &dyn ExecutionClient,
) -> Result<()> {
    // Fetch block and witness
    let (block, witness) = fetch_block_and_witness(rpc_client, block_num).await?;

    // Generate fixture for the block
    let (fixture, fixture_name) =
        generate_fixture(block_num, block, witness, chain_config, chain_name, client.name()).await?;

    // Generate input for the client and save to file
    let result = client.generate_input(&fixture)?;
    result.save_to_file(&fixture_name, output)?;

    Ok(())
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
    let client_name = client.display_name();
    loop {
        if stop.is_cancelled() {
            break;
        }

        // Check for new blocks
        let latest = fetch_latest_block_number(rpc_client).await?;

        for block_num in next_block_num..=latest {
            match process_block(rpc_client, block_num, chain_config, chain_name, output, client).await {
                Ok(_) => {
                    info!("Generated {} input for block: {}", client_name, block_num);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to generate {} input for block {}: {:?}", client_name, block_num, e);
                    error_count += 1;
                }
            }
        }

        next_block_num = latest + 1;

        // Wait before polling again (average block time is ~12s)
        tokio::select! {
            _ = stop.cancelled() => {
                info!("Stopped following blocks");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(6)) => {}
        }
    }

    info!(
        "Completed: {} succeeded, {} failed",
        success_count, error_count
    );

    Ok(())
}

async fn fetch_latest_block_number(client: &HttpClient) -> Result<u64> {
    let block_number = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block,
        Receipt,
        Header,
        TransactionSigned,
    >::block_number(client)
    .await
    .context("Failed to fetch latest block number")?;

    Ok(block_number.to::<u64>())
}

async fn fetch_block_and_witness(
    rpc_client: &HttpClient,
    block_num: u64,
) -> Result<(Block<TransactionSigned>, ExecutionWitness)> {
    let block = EthApiClient::<
        TransactionRequest,
        Transaction,
        Block<TransactionSigned>,
        Receipt,
        Header,
        TransactionSigned,
    >::block_by_number(rpc_client, block_num.into(), true)
    .await
    .context("Failed to fetch block")?
    .with_context(|| format!("Block {} not found", block_num))?;

    let witness = DebugApiClient::<()>::debug_execution_witness(rpc_client, block_num.into())
        .await
        .context("Failed to fetch execution witness for block")?;

    Ok((block, witness))
}

/// Fetch block data and create a fixture for the specified client
async fn generate_fixture(
    block_num: u64,
    block: Block<TransactionSigned>,
    witness: ExecutionWitness,
    chain_config: &ChainConfig,
    chain_name: &str,
    client_name: &str,
) -> Result<(StatelessValidationFixture, String)> {
    // Get transaction count and gas used from the block
    let tx_count = block.transactions.len();
    let gas_used = block.header.gas_used;
    let mgas = gas_used / 1_000_000;

    // Create the fixture
    let fixture_name = format!(
        "{}_{}_{}_{}_zec_{}",
        chain_name.to_lowercase(), block_num, tx_count, mgas, client_name
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

    // Generate reth input
    Ok((fixture, fixture_name))
}

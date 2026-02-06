use alloy_eips::BlockNumberOrTag;
use alloy_genesis::ChainConfig;
use alloy_rpc_types_eth::{Block, Header, Receipt, Transaction, TransactionRequest};
use anyhow::{Context, Result};
use jsonrpsee::http_client::{HeaderMap, HttpClientBuilder};
use stateless_validator_reth::guest::StatelessValidatorRethInput;
use std::path::Path;
use tracing::{info, warn};

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::TransactionSigned;
use reth_rpc_api::{DebugApiClient, EthApiClient};
use reth_stateless::StatelessInput;

use witness_generator::StatelessValidationFixture;

use crate::fixtures::generate_reth_input_from_fixture;
use crate::{OutputFormat, save_reth_input_to_file};

pub async fn reth_input_files_from_rpc(
    rpc_url: &str,
    block: Option<u64>,
    last_n_blocks: Option<usize>,
    rpc_headers: Option<Vec<String>>,
    output: &Path,
    format: OutputFormat,
) -> Result<()> {
    info!("Connecting to RPC: {}", rpc_url);
    let (client, chain_config, chain_name) = init_rpc_client(rpc_url, rpc_headers).await?;

    info!("Connected to chain: {} (ID: {})", chain_name, chain_config.chain_id);

    // Determine which blocks to fetch
    let block_numbers: Vec<u64> = if let Some(block_num) = block {
        vec![block_num]
    } else {
        let n = last_n_blocks.unwrap_or(1);
        if n == 0 {
            info!("No blocks to process (last_n_blocks = 0)");
            return Ok(());
        }
        let latest = fetch_latest_block_number(&client).await?;
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
        match fetch_and_generate_input(&client, block_num, &chain_config, chain_name).await
        {
            Ok((reth_input, fixture_name)) => {
                save_reth_input_to_file(reth_input, &fixture_name, output, format)?;

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

pub async fn reth_input_from_rpc(rpc_url: &str, block_num: u64) -> Result<Vec<u8>> {
    let (client, chain_config, chain_name) = init_rpc_client(rpc_url, None).await?;

    match fetch_and_generate_input(&client, block_num, &chain_config, chain_name).await
    {
        Ok((reth_input, _)) => {
            Ok(bincode::serialize(&reth_input)?)
        }
        Err(e) => {
            anyhow::bail!("Failed to generate input for block {}, error: {:?}", block_num, e);
        }
    }
}

async fn init_rpc_client(rpc_url: &str, rpc_headers: Option<Vec<String>>) -> Result<(jsonrpsee::http_client::HttpClient, ChainConfig, &'static str)> {
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
    let (chain_config, chain_name) = match chain.named() {
        Some(NamedChain::Mainnet) => (mainnet_chain_config(), "mainnet"),
        Some(NamedChain::Sepolia) => (SEPOLIA.genesis.config.clone(), "sepolia"),
        Some(NamedChain::Hoodi) => (HOODI.genesis.config.clone(), "hoodi"),
        Some(NamedChain::Holesky) => (HOLESKY.genesis.config.clone(), "holesky"),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    Ok((client, chain_config, chain_name))
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
    chain_name: &str,
) -> Result<(StatelessValidatorRethInput, String)> {
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

    // Get transaction count and gas used from the block
    let tx_count = block.transactions.len();
    let gas_used = block.header.gas_used;
    let mgas = gas_used / 1_000_000;

    let fixture_name = format!("{}_{}_{}_{}_zec_reth", chain_name, block_num, tx_count, mgas);
    // Create the fixture
    let fixture = StatelessValidationFixture {
        name: fixture_name.clone(),
        stateless_input: StatelessInput {
            block: block.into_consensus(),
            witness,
            chain_config: chain_config.clone(),
        },
        success: true,
    };

    // Generate input
    Ok((generate_reth_input_from_fixture(&fixture)?, fixture_name))
}

    // // Generate input
    // match generate_reth_input_from_fixture(&fixture) {
    //     Ok(reth_input) => {
    //         save_reth_input_to_file(reth_input, &fixture.name, output, format)?;

    //         info!("Generated input for: {}", fixture.name);
    //     }
    //     Err(e) => {
    //         warn!("Failed to generate input for {}: {}", fixture.name, e);
    //     }
    // }
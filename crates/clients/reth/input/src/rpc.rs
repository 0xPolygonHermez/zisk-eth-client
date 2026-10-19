//! Offline stateless input generation from an execution-layer node.
//!
//! Everything comes from the EL: the block, the execution witness and (from
//! Amsterdam) the raw block access list. The EIP-7685 execution requests are
//! not exposed by the EL, so when the header says the block has some, the
//! block is re-executed natively (see [`crate::native`]) to recover them. No
//! consensus-layer access is needed.

use std::time::Instant;

use alloy_consensus::{Block, TxEnvelope};
use alloy_eips::eip7685::EMPTY_REQUESTS_HASH;
use alloy_primitives::{Bytes, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_rpc_types_eth::Block as RpcBlock;
use alloy_transport_http::{
    reqwest::{
        header::{HeaderMap, HeaderName, HeaderValue},
        Client as ReqwestClient,
    },
    Http,
};
use anyhow::{Context, Result};
use stateless_validator_common::guest::input::ProtocolFork;
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    block_input::{self, BlockInput, ExecutionRequests},
    native, RpcConfig,
};

/// Fetch block `block_number` from the EL and build its block input.
pub async fn fetch_block_input(config: &RpcConfig, block_number: u64) -> Result<BlockInput> {
    let provider = connect_provider(config).await?;
    let chain_id = provider.get_chain_id().await.context("eth_chainId")?;

    let start = Instant::now();
    let rpc_block: RpcBlock = provider
        .get_block_by_number(block_number.into())
        .full()
        .await
        .context("eth_getBlockByNumber")?
        .with_context(|| format!("block #{block_number} not found"))?;
    let block_hash: B256 = rpc_block.header.hash;
    let block: Block<TxEnvelope> = rpc_block
        .into_consensus()
        .map_transactions(|tx| tx.into_inner());
    debug!("block {block_number} fetched in {:?}", start.elapsed());

    let fork = block_input::fork_at(chain_id, block.header.timestamp)?;

    let start = Instant::now();
    let witness: ExecutionWitness = provider
        .raw_request("debug_executionWitnessByBlockHash".into(), (block_hash,))
        .await
        .context("debug_executionWitnessByBlockHash")?;
    debug!(
        "witness for block {block_number} fetched in {:?}",
        start.elapsed()
    );

    // Amsterdam: the raw BAL, from the node if it serves it.
    let mut block_access_list: Option<Bytes> = None;
    if fork >= ProtocolFork::Amsterdam {
        match provider
            .raw_request::<_, Option<Bytes>>("eth_getBlockAccessListRaw".into(), (block_hash,))
            .await
        {
            Ok(Some(bal)) => block_access_list = Some(bal),
            Ok(None) => warn!("node has no block access list for block {block_number}"),
            Err(err) => warn!("eth_getBlockAccessListRaw failed for block {block_number}: {err}"),
        }
    }

    // Requests: only recoverable by executing the block.
    let has_requests = block
        .header
        .requests_hash
        .is_some_and(|hash| hash != EMPTY_REQUESTS_HASH);
    let needs_native =
        has_requests || (fork >= ProtocolFork::Amsterdam && block_access_list.is_none());

    let requests = if needs_native {
        let start = Instant::now();
        let executed = native::execute_block(chain_id, fork, &block, &witness)
            .with_context(|| format!("re-executing block {block_number} natively"))?;
        info!(
            "block {block_number} re-executed natively in {:?} (requests: {}, bal: {})",
            start.elapsed(),
            has_requests,
            block_access_list.is_none()
        );
        if block_access_list.is_none() {
            block_access_list = executed.block_access_list;
        }
        executed.requests
    } else {
        ExecutionRequests::default()
    };

    block_input::build_block_input(chain_id, &block, block_access_list, requests, &witness)
}

async fn connect_provider(config: &RpcConfig) -> Result<impl Provider> {
    let url: Url = config.url.parse().context("Invalid RPC URL")?;

    let mut header_map = HeaderMap::new();
    for (k, v) in &config.headers {
        let name = HeaderName::from_bytes(k.as_bytes())
            .with_context(|| format!("Invalid header name: {k}"))?;
        let value =
            HeaderValue::from_str(v).with_context(|| format!("Invalid header value for {k}"))?;
        header_map.insert(name, value);
    }

    let http_client = ReqwestClient::builder()
        .default_headers(header_map)
        .build()
        .context("Failed to build HTTP client")?;

    let http = Http::with_client(http_client, url);
    let rpc_client = RpcClient::new(http, false);
    Ok(ProviderBuilder::new().connect_client(rpc_client))
}

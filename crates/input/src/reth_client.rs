use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::debug;
use url::Url;

use alloy_genesis::ChainConfig;
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
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

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use stateless_reth::StatelessInput;
use zisk_sdk::ZiskStdin;

use guest_common::chain::chain_name;
use guest_reth::{RethInput, RethInputPublic, RethInputWitness};

use super::client::{BlockStats, ExecutionClient, RpcConfig};

#[derive(Default)]
pub struct RethClient;

impl RethClient {
    /// Inherent (not trait-level): only reth can consume a `StatelessInput`,
    /// so this lives outside the [`ExecutionClient`] abstraction.
    pub fn from_stateless_input(&self, stateless_input: &StatelessInput) -> Result<ZiskStdin> {
        let input = RethInput::new(stateless_input)
            .context("Failed to create RethInput from StatelessInput")?;
        self.build_stdin(&input)
    }

    fn build_stdin(&self, input: &RethInput) -> Result<ZiskStdin> {
        let public = RethInputPublic {
            block: input.stateless_input.block.clone(),
            chain_config: input.stateless_input.chain_config.clone(),
            public_keys: input.public_keys.clone(),
        };
        let public_bytes =
            RethInputPublic::serialize(&public).context("Failed to serialize public input")?;
        let witness = RethInputWitness {
            witness: input.stateless_input.witness.clone(),
        };
        let witness_bytes =
            RethInputWitness::serialize(&witness).context("Failed to serialize witness")?;

        let stdin = ZiskStdin::new();
        stdin.write_slice(&public_bytes);
        stdin.write_slice(&witness_bytes);
        Ok(stdin)
    }
}

#[async_trait]
impl ExecutionClient for RethClient {
    fn name(&self) -> &'static str {
        "reth"
    }

    fn display_name(&self) -> &'static str {
        "Reth"
    }

    async fn from_rpc(
        &self,
        config: &RpcConfig,
        block_number: u64,
    ) -> Result<(ZiskStdin, BlockStats)> {
        let provider = connect_provider(config).await?;
        let chain_config = fetch_chain_config(&provider).await?;
        let block = fetch_block(&provider, block_number).await?;
        let witness = fetch_witness(&provider, block_number).await?;

        let stats = BlockStats {
            chain_name: chain_name(chain_config.chain_id),
            block_number,
            tx_count: block.transactions.len(),
            gas_used: block.header.gas_used,
        };

        let stateless_input = StatelessInput {
            block: block.into(),
            witness,
            chain_config,
        };
        let stdin = self
            .from_stateless_input(&stateless_input)
            .with_context(|| format!("Failed to build RethInput for block {block_number}"))?;
        Ok((stdin, stats))
    }

    fn run(&self) {
        guest_reth::run();
    }
}

async fn connect_provider(config: &RpcConfig) -> Result<impl Provider + DebugApi> {
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

    let start = Instant::now();
    let http = Http::with_client(http_client, url);
    let rpc_client = RpcClient::new(http, false);
    let provider = ProviderBuilder::new().connect_client(rpc_client);
    debug!("RPC connect time: {:?}", start.elapsed());
    Ok(provider)
}

async fn fetch_block<P: Provider>(provider: &P, block_number: u64) -> Result<RpcBlock> {
    let start = Instant::now();
    let block = provider
        .get_block(block_number.into())
        .full()
        .await?
        .with_context(|| format!("Block #{block_number} not found"))?;
    debug!(
        "Block fetch time for block {block_number}: {:?}",
        start.elapsed()
    );
    Ok(block)
}

async fn fetch_witness<P: Provider + DebugApi>(
    provider: &P,
    block_number: u64,
) -> Result<ExecutionWitness> {
    let start = Instant::now();
    let witness = provider
        .debug_execution_witness(block_number.into())
        .await
        .context("Failed to fetch execution witness")?;
    debug!(
        "Witness fetch time for block {block_number}: {:?}",
        start.elapsed()
    );
    Ok(witness)
}

async fn fetch_chain_config<P: Provider>(provider: &P) -> Result<ChainConfig> {
    let start = Instant::now();
    let chain_id = provider.get_chain_id().await?;

    let chain = Chain::from_id(chain_id);
    let chain_config = match chain.named() {
        Some(NamedChain::Mainnet) => mainnet_chain_config(),
        Some(NamedChain::Sepolia) => SEPOLIA.genesis.config.clone(),
        Some(NamedChain::Hoodi) => HOODI.genesis.config.clone(),
        Some(NamedChain::Holesky) => HOLESKY.genesis.config.clone(),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    if chain_name(chain_id) == "Unknown" {
        static WARNED: AtomicBool = AtomicBool::new(false);
        if !WARNED.swap(true, Ordering::Relaxed) {
            tracing::warn!("Unrecognized chain ID {chain_id}; output filenames will use 'unknown'");
        }
    }

    debug!("Chain config fetch time: {:?}", start.elapsed());
    Ok(chain_config)
}

use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;
use tracing::debug;

use alloy_genesis::ChainConfig;
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_rpc_types_eth::Block as RpcBlock;

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::{Block, TransactionSigned};

use stateless::{StatelessInput, UncompressedPublicKey};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInput {
    /// The stateless input for the stateless validation function.
    pub stateless_input: StatelessInput,
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

impl RethInput {
    pub fn new(stateless_input: &StatelessInput) -> anyhow::Result<Self> {
        let signers = recover_signers(&stateless_input.block.body.transactions)?;

        Ok(Self {
            stateless_input: stateless_input.clone(),
            public_keys: signers,
        })
    }
}

/// Wrapper for witness part (StatelessInput without public keys)
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInputWitness {
    /// The stateless input (block, witness, chain_config)
    pub stateless_input: StatelessInput,
}

impl RethInputWitness {
    /// Get the block
    pub fn block(&self) -> &Block {
        &self.stateless_input.block
    }

    /// Get the execution witness
    pub fn witness(&self) -> &ExecutionWitness {
        &self.stateless_input.witness
    }

    /// Get the chain config
    pub fn chain_config(&self) -> &ChainConfig {
        &self.stateless_input.chain_config
    }

    /// Fetch witness data from RPC
    pub async fn from_rpc(rpc_url: &str, block_number: u64) -> Result<Self> {
        let provider = connect_provider(rpc_url).await?;
        Self::from_provider(&provider, block_number).await
    }

    /// Fetch witness data using an existing provider
    pub async fn from_provider<P: Provider + DebugApi>(
        provider: &P,
        block_number: u64,
    ) -> Result<Self> {
        let block = fetch_block(provider, block_number).await?;
        let witness = fetch_witness(provider, block_number).await?;
        let chain_config = fetch_chain_config(provider).await?;

        let stateless_input = StatelessInput {
            block: block.into(),
            witness,
            chain_config,
        };

        Ok(Self { stateless_input })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize witness")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize witness")
    }
}
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInputPublic {
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

impl RethInputPublic {
    /// Fetch and recover public keys from RPC
    pub async fn from_rpc(rpc_url: &str, block_number: u64) -> Result<Self> {
        let provider = connect_provider(rpc_url).await?;
        Self::from_provider(&provider, block_number).await
    }

    /// Fetch and recover public keys using an existing provider
    pub async fn from_provider<P: Provider>(provider: &P, block_number: u64) -> Result<Self> {
        let block = fetch_block(provider, block_number).await?;
        let public_keys = Self::public_keys_from_block(&block)?;

        Ok(Self { public_keys })
    }

    /// Recover public keys from a block
    fn public_keys_from_block(block: &RpcBlock) -> Result<Vec<UncompressedPublicKey>> {
        let txs: Vec<TransactionSigned> = block
            .transactions
            .txns()
            .map(|tx| TransactionSigned::from(tx.clone()))
            .collect();

        recover_signers(&txs)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self.public_keys).context("Failed to serialize public keys")
    }

    /// Number of public keys
    pub fn len(&self) -> usize {
        self.public_keys.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.public_keys.is_empty()
    }
}

async fn connect_provider(rpc_url: &str) -> Result<impl Provider + DebugApi> {
    let start_rpc_connect = Instant::now();
    let provider = ProviderBuilder::new()
        .connect(rpc_url)
        .await
        .context("Failed to connect to RPC provider")?;
    let time_rpc_connect = start_rpc_connect.elapsed();
    debug!("RPC connect time: {:?}", time_rpc_connect);
    Ok(provider)
}

async fn fetch_block<P: Provider>(provider: &P, block_number: u64) -> Result<RpcBlock> {
    let start_block_fetch = Instant::now();
    let block = provider
        .get_block(block_number.into())
        .full()
        .await?
        .with_context(|| format!("Block #{block_number} not found"))?;
    let time_block_fetch = start_block_fetch.elapsed();
    debug!(
        "Block fetch time for block {block_number}: {:?}",
        time_block_fetch
    );
    Ok(block)
}

async fn fetch_witness<P: Provider + DebugApi>(
    provider: &P,
    block_number: u64,
) -> Result<ExecutionWitness> {
    let start_witness_fetch = Instant::now();
    let witness = provider
        .debug_execution_witness(block_number.into())
        .await
        .context("Failed to fetch execution witness")?;
    let time_witness_fetch = start_witness_fetch.elapsed();
    debug!(
        "Witness fetch time for block {block_number}: {:?}",
        time_witness_fetch
    );
    Ok(witness)
}

async fn fetch_chain_config<P: Provider>(provider: &P) -> Result<ChainConfig> {
    let start_chain_config_fetch = Instant::now();
    let chain_id = provider.get_chain_id().await?;

    let chain = Chain::from_id(chain_id);
    let chain_config = match chain.named() {
        Some(NamedChain::Mainnet) => mainnet_chain_config(),
        Some(NamedChain::Sepolia) => SEPOLIA.genesis.config.clone(),
        Some(NamedChain::Hoodi) => HOODI.genesis.config.clone(),
        Some(NamedChain::Holesky) => HOLESKY.genesis.config.clone(),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    let time_chain_config_fetch = start_chain_config_fetch.elapsed();
    debug!("Chain config fetch time: {:?}", time_chain_config_fetch);
    Ok(chain_config)
}

// Recovers the signing [`UncompressedPublicKey`] from each transaction's signature, in parallel.
fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<UncompressedPublicKey>> {
    txs.par_iter()
        .enumerate()
        .map(|(i, tx)| {
            let keys = tx
                .signature()
                .recover_from_prehash(&tx.signature_hash())
                .with_context(|| format!("Failed to recover signature for tx #{i}"))?;

            let encoded_point: [u8; 65] = keys
                .to_encoded_point(false)
                .as_bytes()
                .try_into()
                .map_err(|e| anyhow!("Failed to encode public key for tx #{i}, error: {e}"))?;

            Ok(UncompressedPublicKey(encoded_point))
        })
        .collect()
}

/// Fetch full input from RPC
pub async fn reth_input_from_rpc(rpc_url: &str, block_number: u64) -> Result<RethInput> {
    let provider = connect_provider(rpc_url).await?;

    let block = fetch_block(&provider, block_number).await?;
    let witness = fetch_witness(&provider, block_number).await?;
    let chain_config = fetch_chain_config(&provider).await?;

    let stateless_input = StatelessInput {
        block: block.into(),
        witness: witness.clone(),
        chain_config: chain_config.clone(),
    };

    RethInput::new(&stateless_input).context("Failed to create RethInput")
}

use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;
use tracing::debug;

use alloy_genesis::ChainConfig;
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_rpc_types_eth::Block;

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::TransactionSigned;

use stateless::{StatelessInput, UncompressedPublicKey};

use stateless_validator_common::new_payload_request::NewPayloadRequest;
use stateless_validator_reth::guest::StatelessValidatorRethInput;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorRethInputWitness {
    /// New payload request data.
    pub new_payload_request: NewPayloadRequest,
    /// Execution witness for the EL block.
    pub witness: ExecutionWitness,
    /// Chain configuration for the stateless validation function
    #[serde_as(as = "alloy_genesis::serde_bincode_compat::ChainConfig<'_>")]
    pub chain_config: ChainConfig,
}

impl StatelessValidatorRethInputWitness {
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
            witness: witness.clone(),
            chain_config: chain_config.clone(),
        };
        let reth_input = reth_input_from_stateless(&stateless_input, true)?;

        Ok(Self {
            new_payload_request: reth_input.new_payload_request,
            witness,
            chain_config,
        })
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorRethInputPk {
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

impl StatelessValidatorRethInputPk {
    /// Fetch and recover public keys from RPC
    pub async fn from_rpc(rpc_url: &str, block_number: u64) -> Result<Self> {
        let provider = connect_provider(rpc_url).await?;
        Self::from_provider(&provider, block_number).await
    }

    /// Fetch and recover public keys using an existing provider
    pub async fn from_provider<P: Provider>(provider: &P, block_number: u64) -> Result<Self> {
        let block = fetch_block(provider, block_number).await?;
        Self::from_block(&block)
    }

    /// Recover public keys from a block
    pub fn from_block(block: &Block) -> Result<Self> {
        let txs: Vec<TransactionSigned> = block
            .transactions
            .txns()
            .map(|tx| TransactionSigned::from(tx.clone()))
            .collect();

        let public_keys = recover_signers(&txs)?;
        Ok(Self { public_keys })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self.public_keys).context("Failed to serialize public keys")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let public_keys =
            bincode::deserialize(bytes).context("Failed to deserialize public keys")?;
        Ok(Self { public_keys })
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

/// Fetch full input from RPC
pub async fn reth_input_from_rpc(
    rpc_url: &str,
    block_number: u64,
) -> Result<StatelessValidatorRethInput> {
    let provider = connect_provider(rpc_url).await?;

    let block = fetch_block(&provider, block_number).await?;
    let witness = fetch_witness(&provider, block_number).await?;
    let chain_config = fetch_chain_config(&provider).await?;

    let stateless_input = StatelessInput {
        block: block.into(),
        witness: witness.clone(),
        chain_config: chain_config.clone(),
    };

    reth_input_from_stateless(&stateless_input, true)
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

async fn fetch_block<P: Provider>(provider: &P, block_number: u64) -> Result<Block> {
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
    let chain_config = get_chain_config(chain_id)?;
    let time_chain_config_fetch = start_chain_config_fetch.elapsed();
    debug!("Chain config fetch time: {:?}", time_chain_config_fetch);
    Ok(chain_config)
}

/// Get chain config and name from chain ID
fn get_chain_config(chain_id: u64) -> Result<ChainConfig> {
    let chain = Chain::from_id(chain_id);
    match chain.named() {
        Some(NamedChain::Mainnet) => Ok(mainnet_chain_config()),
        Some(NamedChain::Sepolia) => Ok(SEPOLIA.genesis.config.clone()),
        Some(NamedChain::Hoodi) => Ok(HOODI.genesis.config.clone()),
        Some(NamedChain::Holesky) => Ok(HOLESKY.genesis.config.clone()),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    }
}

/// Generate reth input from a StatelessInput
fn reth_input_from_stateless(
    stateless_input: &StatelessInput,
    success: bool,
) -> Result<StatelessValidatorRethInput> {
    StatelessValidatorRethInput::new(stateless_input, success)
        .context("Failed to create StatelessValidatorRethInput")
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

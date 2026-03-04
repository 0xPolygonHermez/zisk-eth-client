use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tracing::debug;

use alloy_genesis::ChainConfig;
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use alloy_rpc_types_debug::ExecutionWitness;

use reth_chainspec::{mainnet_chain_config, Chain, NamedChain, HOLESKY, HOODI, SEPOLIA};
use reth_ethereum_primitives::TransactionSigned;

use stateless::{StatelessInput, UncompressedPublicKey};

use stateless_validator_common::new_payload_request::NewPayloadRequest;
use stateless_validator_reth::guest::StatelessValidatorRethInput;

/// StatelessValidatorRethInput with public keys split out
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorRethInputNoPk {
    /// New payload request data.
    pub new_payload_request: NewPayloadRequest,
    /// Execution witness for the EL block.
    pub witness: ExecutionWitness,
    /// Chain configuration for the stateless validation function
    #[serde_as(as = "alloy_genesis::serde_bincode_compat::ChainConfig<'_>")]
    pub chain_config: ChainConfig,
}

pub async fn reth_input_from_rpc(
    rpc_url: &str,
    block_number: u64,
) -> Result<StatelessValidatorRethInput> {
    let stateless_input = fetch_stateless_input(rpc_url, block_number).await?;
    reth_input_from_stateless(&stateless_input, true)
}

/// Serialize only the public keys from a reth input
pub fn serialize_public_keys(reth_input: &StatelessValidatorRethInput) -> Result<Vec<u8>> {
    bincode::serialize(&reth_input.public_keys).context("Failed to serialize public keys")
}

/// Serialize reth input without public keys
pub fn serialize_reth_input_no_pk(reth_input: StatelessValidatorRethInput) -> Result<Vec<u8>> {
    let input_no_pk = StatelessValidatorRethInputNoPk {
        new_payload_request: reth_input.new_payload_request,
        witness: reth_input.witness,
        chain_config: reth_input.chain_config,
    };
    bincode::serialize(&input_no_pk).context("Failed to serialize reth input (no pk)")
}

/// Fetch block data from RPC and create a StatelessInput
async fn fetch_stateless_input(rpc_url: &str, block_number: u64) -> Result<StatelessInput> {
    let start_rpc_connect = std::time::Instant::now();
    let provider = ProviderBuilder::new().connect(rpc_url).await?;
    let time_rpc_connect = start_rpc_connect.elapsed();

    // Get the block
    let start_block_fetch = std::time::Instant::now();
    let block = provider
        .get_block(block_number.into())
        .full()
        .await?
        .with_context(|| format!("Block #{block_number} not found"))?;
    let time_block_fetch = start_block_fetch.elapsed();

    // Get the execution witness
    let start_witness_fetch = std::time::Instant::now();
    let witness = provider
        .debug_execution_witness(block_number.into())
        .await?;
    let time_witness_fetch = start_witness_fetch.elapsed();

    // Get the chain config
    let chain_id = provider.get_chain_id().await?;
    let chain_config = get_chain_config(chain_id)?;

    debug!(
        "RPC timings for block {block_number}: connect: {:?}, block: {:?}, witness: {:?}",
        time_rpc_connect, time_block_fetch, time_witness_fetch
    );

    Ok(StatelessInput {
        block: block.into(),
        witness,
        chain_config,
    })
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
pub fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<UncompressedPublicKey>> {
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

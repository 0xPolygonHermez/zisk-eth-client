use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use anyhow::{anyhow, Context, Result};
use stateless_validator_reth::guest::StatelessValidatorRethInput;
use rayon::prelude::*;
use reth_chainspec::{Chain, HOLESKY, HOODI, NamedChain, SEPOLIA, mainnet_chain_config};
use reth_ethereum_primitives::TransactionSigned;
use reth_stateless::{StatelessInput, UncompressedPublicKey};
use tracing::debug;

pub async fn reth_input_from_rpc(rpc_url: &str, block_number: u64) -> anyhow::Result<Vec<u8>> {
    let start_rpc_connect = std::time::Instant::now();
    let provider = ProviderBuilder::new()
        .connect(rpc_url)
        .await?;
    let time_rpc_connect = start_rpc_connect.elapsed();

    let start_block_fetch = std::time::Instant::now();
    let rpc_block = provider
        .get_block(block_number.into())
        .full()
        .await?
        .with_context(|| format!("block {block_number} not found"))?;
    let time_block_fetch = start_block_fetch.elapsed();

    let start_witness_fetch = std::time::Instant::now();
    let witness = provider
        .debug_execution_witness(rpc_block.number().into())
        .await?;
    let time_witness_fetch = start_witness_fetch.elapsed();

    let start_serialize_input = std::time::Instant::now();

    let block = reth_ethereum_primitives::Block::from(rpc_block);

    // Get chain config
    let chain_id = provider.get_chain_id().await?;
    let chain = Chain::from_id(chain_id);
    let (chain_config, chain_name) = match chain.named() {
        Some(NamedChain::Mainnet) => (mainnet_chain_config(), "mainnet"),
        Some(NamedChain::Sepolia) => (SEPOLIA.genesis.config.clone(), "sepolia"),
        Some(NamedChain::Hoodi) => (HOODI.genesis.config.clone(), "hoodi"),
        Some(NamedChain::Holesky) => (HOLESKY.genesis.config.clone(), "holesky"),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    };

    let tx_count = block.body.transactions.len();
    let gas_used = block.header.gas_used;
    let mgas = gas_used / 1_000_000;

    // Create the fixture
    let fixture_name = format!(
        "{}_{}_{}_{}_zec_reth",
        chain_name, block_number, tx_count, mgas
    );
    let stateless_input = StatelessInput {
        block,
        witness,
        chain_config: chain_config.clone(),
    };

    // Generate reth input
    let reth_input = StatelessValidatorRethInput::new(&stateless_input, true)
        .with_context(|| {
            format!(
                "Failed to create StatelessValidatorReth input for {}",
                fixture_name
            )
        })?;

    let input_bytes = bincode::serialize(&reth_input)
        .with_context(|| format!("Failed to serialize reth input for block {}", block_number))?;

    let time_serialize_input = start_serialize_input.elapsed();

    debug!("Input generation timings for block {block_number}: rpc connect: {:?}, block fetch: {:?}, witness fetch: {:?}, serialize input: {:?}",
        time_rpc_connect,
        time_block_fetch,
        time_witness_fetch,
        time_serialize_input,
    );

    Ok(input_bytes)
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

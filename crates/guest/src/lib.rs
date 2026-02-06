use std::sync::Arc;

use alloy_genesis::Genesis;
use alloy_primitives::B256;
use sparsestate::SparseState;
use stateless_validator_common::new_payload_request::NewPayloadRequest;
use stateless_validator_reth::{
    guest::StatelessValidatorRethInput, new_payload_request::new_payload_request_to_block,
};
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{stateless_validation_with_trie, validation::StatelessValidationError};

/// Performs stateless validation of a block using the provided witness data.
pub fn validate_block(
    input: StatelessValidatorRethInput,
) -> Result<B256, StatelessValidationError> {
    // Build chain spec from input's chain config
    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    // Convert new payload request to block
    let block = new_payload_request_to_block(input.new_payload_request, chain_spec.clone())
        .map_err(|err| {
            println!("Failed to convert to reth block: {err}");
            StatelessValidationError::Custom("Block conversion failed")
        })?
        .into_block();

    // Perform stateless validation
    let (hash, _) = stateless_validation_with_trie::<SparseState, _, _>(
        block,
        input.public_keys,
        input.witness,
        chain_spec,
        evm_config,
    )?;

    Ok(hash)
}

/// Get chain name from chain ID
pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        0x1 => "Mainnet",
        0xaa36a7 => "Sepolia",
        0x4268 => "Holesky",
        0x5 => "Goerli",
        _ => "Unknown",
        // Add more chain IDs as needed
    }
}

/// Extract common execution payload information across forks.
pub fn extract_block_info(req: &NewPayloadRequest) -> (u64, u64, usize) {
    match req {
        NewPayloadRequest::Bellatrix(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::Capella(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::Deneb(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::ElectraFulu(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
    }
}

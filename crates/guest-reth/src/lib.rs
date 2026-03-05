use std::sync::Arc;

use alloy_genesis::Genesis;
use alloy_primitives::B256;

use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;

use stateless_reth::{stateless_validation_with_trie, validation::StatelessValidationError};
use zeth_mpt_state::SparseState;

use stateless_validator_reth::{
    guest::StatelessValidatorRethInput, new_payload_request::new_payload_request_to_block,
};

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

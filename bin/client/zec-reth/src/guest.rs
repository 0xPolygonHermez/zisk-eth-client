use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::sync::Arc;

use crypto::CustomEvmCrypto;

use alloy_consensus::crypto::install_default_provider;
use alloy_genesis::{ChainConfig, Genesis};
use alloy_primitives::B256;

use sparsestate::SparseState;

use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{validation::StatelessValidationError, ExecutionWitness, UncompressedPublicKey, stateless_validation_with_trie};

use revm::install_crypto;

use stateless_validator_common::new_payload_request::NewPayloadRequest;
use stateless_validator_reth::new_payload_request::new_payload_request_to_block;

// TODO: Import it from witness_generator when erorrs are resolved
/// Input for the stateless validator guest program.
/// Copied from https://github.com/eth-act/ere-guests/blob/main/crates/stateless-validator-reth/src/guest.rs
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatelessValidatorRethInput {
    /// New payload request data.
    pub new_payload_request: NewPayloadRequest,
    /// Execution witness for the EL block.
    pub witness: ExecutionWitness,
    /// Chain configuration for the stateless validation function
    #[serde_as(as = "alloy_genesis::serde_bincode_compat::ChainConfig<'_>")]
    pub chain_config: ChainConfig,
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

/// Performs stateless validation of a block using the provided witness data.
pub fn validate_block(input: StatelessValidatorRethInput) -> Result<B256, StatelessValidationError> {
    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

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

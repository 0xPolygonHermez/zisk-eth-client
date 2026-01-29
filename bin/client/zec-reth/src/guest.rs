use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::sync::Arc;

use crypto::CustomEvmCrypto;

use alloy_consensus::crypto::install_default_provider;
use alloy_genesis::{ChainConfig, Genesis};

use sparsestate::SparseState;

use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{ExecutionWitness, UncompressedPublicKey, stateless_validation_with_trie};

use revm::install_crypto;

use stateless_validator_common::{
    guest::StatelessValidatorOutput, new_payload_request::NewPayloadRequest,
};
use stateless_validator_reth::new_payload_request::new_payload_request_to_block;

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
pub fn validate_block(input: StatelessValidatorRethInput) -> StatelessValidatorOutput {
    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    let new_payload_request_root = input.new_payload_request.tree_hash_root();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        validate_block_inner(input, new_payload_request_root)
    }));

    match result {
        Ok(output) => output,
        Err(_) => {
            println!("Panic occurred during validation\n");
            StatelessValidatorOutput::new(new_payload_request_root, false)
        }
    }
}

fn validate_block_inner(
    input: StatelessValidatorRethInput,
    new_payload_request_root: [u8; 32],
) -> StatelessValidatorOutput {
    // Build chain spec from input's chain config
    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    // Convert new payload request to block
    let block = match new_payload_request_to_block(input.new_payload_request, chain_spec.clone()) {
        Ok(sealed_block) => sealed_block.into_block(),
        Err(err) => {
            println!("Failed to convert to reth block: {err}");
            return StatelessValidatorOutput::new(new_payload_request_root, false);
        }
    };

    // Perform stateless validation
    let result = stateless_validation_with_trie::<SparseState, _, _>(
        block,
        input.public_keys,
        input.witness,
        chain_spec,
        evm_config,
    );

    match result {
        Ok(_) => StatelessValidatorOutput::new(new_payload_request_root, true),
        Err(err) => {
            println!("Block validation failed: {err}");
            StatelessValidatorOutput::new(new_payload_request_root, false)
        }
    }
}

use std::sync::Arc;

use alloy_genesis::{ChainConfig, Genesis};
use alloy_primitives::B256;
use alloy_rpc_types_debug::ExecutionWitness;

use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::Block;
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives_traits::RecoveredBlock;
use stateless::{
    recover_block_with_public_keys, stateless_validation_recovered_with_trie,
    validation::StatelessValidationError, UncompressedPublicKey,
};
use zeth_mpt_state::SparseState;

use stateless_validator_common::new_payload_request::NewPayloadRequest;

/// Verifies transaction signatures against provided public keys.
pub fn verify_signatures(
    block: Block,
    chain_spec: Arc<ChainSpec>,
    public_keys: Vec<UncompressedPublicKey>,
) -> Result<RecoveredBlock<Block>, StatelessValidationError> {
    // Recover block with public keys while validating signatures
    let recovered_block = recover_block_with_public_keys(block, public_keys, &*chain_spec)?;

    Ok(recovered_block)
}

/// Performs stateless validation of a block using pre-verified signatures.
pub fn validate_block_stateless(
    recovered_block: RecoveredBlock<Block>,
    witness: ExecutionWitness,
    chain_spec: Arc<ChainSpec>,
) -> Result<B256, StatelessValidationError> {
    // Create EVM config from chain spec
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    // Perform stateless validation
    let (hash, _) = stateless_validation_recovered_with_trie::<SparseState, _, _>(
        recovered_block,
        witness,
        chain_spec,
        evm_config,
    )?;

    Ok(hash)
}

pub fn get_chain_spec(chain_config: ChainConfig) -> Arc<ChainSpec> {
    // Build chain spec from chain config
    let genesis = Genesis {
        config: chain_config,
        ..Default::default()
    };
    Arc::new(genesis.into())
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

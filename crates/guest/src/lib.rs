use std::sync::Arc;

use alloy_genesis::Genesis;
use alloy_primitives::B256;

use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;

use primitive_types::H256;

use ethrex_stateless::{execution::execution_program, input::ProgramInput};
use reth_stateless::{stateless_validation_with_trie, validation::StatelessValidationError};

use sparsestate::SparseState;
use stateless_validator_common::new_payload_request::NewPayloadRequest;
use stateless_validator_ethrex::{
    guest::StatelessValidatorEthrexInput, new_payload_request::get_block_from_new_payload_request,
};
use stateless_validator_reth::{
    guest::StatelessValidatorRethInput, new_payload_request::new_payload_request_to_block,
};

/// Performs stateless validation of a block using the provided witness data (Reth).
pub fn validate_block_reth(
    input: StatelessValidatorRethInput,
) -> Result<B256, StatelessValidationError> {
    // Build chain spec from input's chain config
    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    // Convert new payload request to reth block
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

/// Performs stateless validation of a block using the provided witness data (Ethrex).
pub fn validate_block_ethrex(
    input: StatelessValidatorEthrexInput,
) -> Result<H256, StatelessValidationError> {
    // Convert new payload request to ethrex block
    let block = get_block_from_new_payload_request(input.new_payload_request).map_err(|err| {
        println!("Failed to convert to ethrex block: {err}");
        StatelessValidationError::Custom("Block construction failed")
    })?;

    let block_num = block.header.number;

    // Build program input
    let program_input = ProgramInput {
        blocks: vec![block],
        execution_witness: input.execution_witness,
        elasticity_multiplier: input.elasticity_multiplier,
        fee_configs: input.fee_configs,
    };

    // Perform stateless validation
    let res = execution_program(program_input).map_err(|err| {
        println!(
            "Failed to execute ethrex program for block {}: {err}",
            block_num
        );
        StatelessValidationError::Custom("Ethrex execution failed")
    })?;

    Ok(res.final_state_hash)
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

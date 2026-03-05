use primitive_types::{H256, U256};

use stateless_ethrex::{
    execution::{execution_program, StatelessExecutionError},
    input::ProgramInput,
};

use stateless_validator_ethrex::{
    guest::StatelessValidatorEthrexInput, new_payload_request::get_block_from_new_payload_request,
};

/// Performs stateless validation of a block using the provided witness data (Ethrex).
pub fn validate_block(
    input: StatelessValidatorEthrexInput,
) -> Result<(U256, H256), StatelessExecutionError> {
    // Convert new payload request to ethrex block
    let block = get_block_from_new_payload_request(input.new_payload_request).map_err(|e| {
        StatelessExecutionError::Internal(format!("Block construction failed: {}", e))
    })?;

    // Build program input
    let program_input = ProgramInput {
        blocks: vec![block],
        execution_witness: input.execution_witness,
        elasticity_multiplier: input.elasticity_multiplier,
        fee_configs: input.fee_configs,
    };

    // Perform stateless validation
    let res = execution_program(program_input)?;

    Ok((res.chain_id, res.last_block_hash))
}

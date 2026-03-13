use primitive_types::H256;
use std::sync::Arc;

use ethrex_crypto::Crypto;
use stateless_ethrex::{
    common::ExecutionError,
    execution::execution_program,
    input::ProgramInput,
    output::ProgramOutput,
};

use super::EthrexInput;

/// Performs stateless validation of a block using the provided witness data
pub fn validate_block(input: EthrexInput, crypto: Arc<dyn Crypto>) -> Result<H256, ExecutionError> {
    // Build program input
    let input = ProgramInput {
        blocks: vec![input.block().clone()],
        execution_witness: input.witness().clone(),
    };

    // Perform stateless validation
    let ProgramOutput {
        last_block_hash,
        ..
    } = execution_program(input, crypto)?;

    Ok(last_block_hash)
}

use primitive_types::H256;
use std::sync::Arc;

use ethrex_common::types::ELASTICITY_MULTIPLIER;
use ethrex_crypto::Crypto;
use ethrex_vm::Evm;
use stateless_ethrex::common::{execute_blocks, BatchExecutionResult, ExecutionError};
// use stateless_ethrex::{
//     common::{execute_blocks, BatchExecutionResult, ExecutionError},
//     execution::execution_program,
//     input::ProgramInput,
// };

use super::EthrexInput;

/// Performs stateless validation of a block using the provided witness data (Ethrex).
pub fn validate_block(input: EthrexInput, crypto: Arc<dyn Crypto>) -> Result<H256, ExecutionError> {
    // TODO: Substitute the remaining code with this function call when it gets solved in ethrex
    // // Build program input
    // let program_input = ProgramInput {
    //     blocks: vec![input.block().clone()],
    //     execution_witness: input.witness().clone(),
    // };

    // // Perform stateless validation
    // let res = execution_program(program_input)?;

    let blocks = vec![input.block().clone()];
    let execution_witness = input.witness().clone();
    let BatchExecutionResult {
        receipts: _,
        initial_state_hash: _,
        final_state_hash: _,
        last_block_hash,
        non_privileged_count: _,
        chain_id: _,
    } = execute_blocks(
        &blocks,
        execution_witness,
        ELASTICITY_MULTIPLIER,
        |db, _| {
            // L1 VM factory - simple creation without fee configs
            Ok(Evm::new_for_l1(db.clone(), crypto.clone()))
        },
    )?;

    Ok(last_block_hash)
}

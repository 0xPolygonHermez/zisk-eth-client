use primitive_types::H256;
use std::sync::Arc;

use ethrex_crypto::Crypto;
use stateless_ethrex::{common::ExecutionError, l1::validate_blocks_statelessly};

use super::EthrexInput;

/// Performs stateless validation of a block using the provided witness data
pub fn validate_block(input: EthrexInput, crypto: Arc<dyn Crypto>) -> Result<H256, ExecutionError> {
    // ethrex's L1 spec entrypoint takes SSZ `statelessInputBytes`, which is
    // Amsterdam-only; this takes a block and its witness, as ours does.
    let EthrexInput {
        block,
        execution_witness,
    } = input;
    let blocks = [block];

    // Perform stateless validation
    validate_blocks_statelessly(&blocks, execution_witness, crypto)?;

    // Execution already computed and cached the header hash.
    Ok(blocks[0].hash())
}

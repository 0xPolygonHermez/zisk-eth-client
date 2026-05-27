use std::sync::Arc;

use guest_common::chain::chain_name;
use ziskos::io::read_input_slice;

use super::{extract_block_info, validate_block, EthrexInput, ZiskCrypto};

pub fn run() {
    // Read the input
    let input: EthrexInput =
        EthrexInput::deserialize(&read_input_slice()).expect("Failed to deserialize EthrexInput");

    // Get chain config
    let chain_config = input.witness().chain_config;

    // Extract useful information for logging
    let block = input.block();
    let (block_number, gas_used, tx_count) = extract_block_info(block);
    let chain_id = chain_config.chain_id;
    let chain = chain_name(chain_id);
    println!(
        "Executing block validation for {} Block #{} ({} txs)",
        chain, block_number, tx_count
    );

    // Validate the block
    let crypto = ZiskCrypto::default();
    let block_hash = validate_block(input, Arc::new(crypto)).expect("Block validation failed");

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Print block number and calculated hash
    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {:?}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}

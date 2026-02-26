#![no_main]
ziskos::entrypoint!(main);

use guest::{chain_name, extract_block_info, validate_block_ethrex};
use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexInput, StatelessValidatorEthrexIo,
};

use ere_io::Io;

fn main() {
    // Read and deserialize input
    let input: Vec<u8> = ziskos::io::read_vec();
    let input: StatelessValidatorEthrexInput =
        StatelessValidatorEthrexIo::deserialize_input(&input).expect("Failed to deserialize input");

    // Get chain info
    // let chain_id = input.chain_config.chain_id; // TODO
    let chain_id = 1u64;
    let chain = chain_name(chain_id);

    // Extract useful information for logging
    let (block_number, gas_used, tx_count) = extract_block_info(&input.new_payload_request);

    // Validate the block
    println!(
        "Executing block validation for {} Block #{} ({} txs)",
        chain, block_number, tx_count
    );
    let block_hash = validate_block_ethrex(input).expect("Block validation failed");

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Print block number and calculated hash
    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}

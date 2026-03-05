#![no_main]
ziskos::entrypoint!(main);

use ere_io::Io;
use guest_common::{chain_name, extract_block_info};
use guest_ethrex::validate_block;
use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexInput, StatelessValidatorEthrexIo,
};

fn main() {
    // Read and deserialize input
    let input: Vec<u8> = ziskos::io::read_vec();
    let input: StatelessValidatorEthrexInput =
        StatelessValidatorEthrexIo::deserialize_input(&input).expect("Failed to deserialize input");

    // Extract useful information for logging
    let (block_number, gas_used, tx_count) = extract_block_info(&input.new_payload_request);

    // Validate the block
    println!(
        "Executing Ethrex block validation for Block #{} ({} txs)",
        block_number, tx_count
    );
    let (chain_id, block_hash) = validate_block(input).expect("Block validation failed");

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Get chain info
    let chain = chain_name(chain_id.low_u64());

    // Print some stats
    println!("Block validation succeeded!");
    println!(
        "Ethrex execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {:?}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}

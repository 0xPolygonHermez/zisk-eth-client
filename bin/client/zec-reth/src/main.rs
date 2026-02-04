#![no_main]
ziskos::entrypoint!(main);

use ziskos::{read_input_slice, set_output};

use stateless_validator_reth::guest::StatelessValidatorRethInput;

mod guest;

use guest::{chain_name, extract_block_info, validate_block};

fn main() {
    // Read and deserialize input
    let input = read_input_slice();
    let input: StatelessValidatorRethInput =
        bincode::deserialize(&input).expect("Failed to deserialize input");

    // Get chain info
    let chain_id = input.chain_config.chain_id;
    let chain = chain_name(chain_id);

    // Extract useful information for logging
    let (block_number, gas_used, tx_count) = extract_block_info(&input.new_payload_request);

    // Validate the block
    println!(
        "Executing block validation for {} Block #{} ({} txs)",
        chain, block_number, tx_count
    );
    let block_hash = validate_block(input).expect("Block validation failed");

    // Write block_hash value to the public output
    for (i, chunk) in block_hash.to_vec().chunks_exact(4).enumerate() {
        let limb = u32::from_le_bytes(chunk.try_into().unwrap());
        set_output(i, limb);
    }

    // Print block number and calculated hash
    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  -Chain: {} (ID: {})\n  -Block Number: {}\n  -Data Hash: {}\n  -Transaction Count: {}\n  -Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}

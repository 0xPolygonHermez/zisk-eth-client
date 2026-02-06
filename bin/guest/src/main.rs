#![no_main]
ziskos::entrypoint!(main);

use std::sync::Arc;

use guest::{chain_name, extract_block_info, validate_block};
use stateless_validator_reth::guest::StatelessValidatorRethInput;
use alloy_consensus::crypto::install_default_provider;
use crypto::CustomEvmCrypto;
use revm::install_crypto;

fn main() {
    #[cfg(zisk_hints)]
    {
        // Create ./hints directory if it doesn't exist
        let hints_dir = std::path::PathBuf::from("./hints");
        if !hints_dir.exists() {
            std::fs::create_dir_all(&hints_dir).expect("Failed to create hints directory");
        }
        // Initialize hints file
        let hints_file = std::path::PathBuf::from("./hints/block_hints.bin");
        if let Err(e) = ziskos::hints::init_hints_file(hints_file) {
            panic!("Failed to init hints, error: {}", e);
        }
    }

    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    // Read and deserialize input
    let input: StatelessValidatorRethInput = ziskos::io::read();

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

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Print block number and calculated hash
    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  -Chain: {} (ID: {})\n  -Block Number: {}\n  -Data Hash: {}\n  -Transaction Count: {}\n  -Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );

    #[cfg(zisk_hints)]
    {
        // Close hints generation
        if let Err(e) = ziskos::hints::close_hints() {
            panic!("Failed to close hints, error: {}", e);
        }

        // Rename hint file
        let hints_file = std::path::PathBuf::from("./hints/block_hints.bin");
        let new_hints_file = std::path::PathBuf::from(format!("./hints/{}_hints.bin", block_number));
        std::fs::rename(&hints_file, &new_hints_file).unwrap();

        println!("Hints generated successfully in file {}", &new_hints_file.display());
    }
}

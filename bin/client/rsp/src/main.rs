#![no_main]
ziskos::entrypoint!(main);

use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;

use rsp_client_executor::{executor::EthClientExecutor, io::EthClientExecutorInput};

use crypto::CustomEvmCrypto;

fn main() {
    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    let input: EthClientExecutorInput = ziskos::io::read();

    let block_number = input.current_block.number;

    println!("Executing {} block", block_number);

    // Execute the block.
    let executor = EthClientExecutor::eth(
        Arc::new(
            (&input.genesis)
                .try_into()
                .expect("Failed to convert genesis block into the required type"),
        ),
        input.custom_beneficiary,
    );
    let header = executor.execute(input).expect("Failed to execute client");

    // Calculate block hash
    let block_hash = header.hash_slow();

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Print block number and calculated hash
    println!("Block number: {}, hash: {}", block_number, block_hash);
}

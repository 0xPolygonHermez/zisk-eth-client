#![no_main]
ziskos::entrypoint!(main);

use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;

use zeth_chainspec::MAINNET;
use zeth_core::{validate_block, EthEvmConfig, Input};

use crypto::CustomEvmCrypto;

fn main() {
    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    let input: Input = ziskos::io::read();

    let block_number = input.block.header.number;

    println!("Executing {} block", block_number);

    let evm_config = EthEvmConfig::new(MAINNET.clone());

    let block_hash = validate_block(input.clone(), evm_config).expect("Failed to validate block");

    // Commit to block hash as the output
    ziskos::io::commit(&block_hash);

    // Print block number and calculated hash
    println!("Block number: {}, hash: {}", block_number, block_hash);
}

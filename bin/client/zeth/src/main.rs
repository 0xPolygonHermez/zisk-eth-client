#![no_main]
ziskos::entrypoint!(main);

use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use crypto::CustomEvmCrypto;
use revm::install_crypto;
use zeth_chainspec::MAINNET;
use zeth_core::{validate_block, EthEvmConfig, Input};
use ziskos::{read_input_slice, set_output};

//TODO: Check why ecrecover not being patched correctly for tx recovery (alloy)

fn main() {
    // Install custom EVM crypto
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    let input = read_input_slice();

    let input = bincode::deserialize::<Input>(&input).unwrap();
    let block_number = input.block.header.number;

    println!("Executing {} block", block_number);

    let evm_config = EthEvmConfig::new(MAINNET.clone());

    let block_hash = validate_block(input.clone(), evm_config).expect("Failed to validate block");

    // Write block_hash value to the public output
    for (index, chunk) in block_hash.to_vec().chunks(4).enumerate() {
        let value = u32::from_le_bytes(chunk.try_into().unwrap());
        set_output(index, value);
    }

    // Print block number and calculated hash
    println!("Block number: {}, hash: {}", block_number, block_hash);
}

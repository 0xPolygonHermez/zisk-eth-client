#![no_main]
ziskos::entrypoint!(main);

mod hashes;
use hashes::{blake2f_tests, sha256_tests, keccak256_tests};

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    blake2f_tests(&reth_crypto);
    sha256_tests(&reth_crypto);
    keccak256_tests();
}
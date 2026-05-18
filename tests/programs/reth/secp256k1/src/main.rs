#![no_main]
ziskos::entrypoint!(main);

mod secp256k1;
use secp256k1::{ecrecover_precompile_tests, ecrecover_tx_tests};

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    // Secp256k1
    ecrecover_tx_tests(&reth_crypto);
    ecrecover_precompile_tests(&reth_crypto);
}
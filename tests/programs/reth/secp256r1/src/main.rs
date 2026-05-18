#![no_main]
ziskos::entrypoint!(main);

mod secp256r1;
use secp256r1::p256_verify_tests;

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    p256_verify_tests(&reth_crypto);
}

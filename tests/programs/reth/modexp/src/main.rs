#![no_main]
ziskos::entrypoint!(main);

mod modexp;
use modexp::modexp_tests;

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    modexp_tests(&reth_crypto);
}
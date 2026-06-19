#![no_main]
ziskos::entrypoint!(main);

mod bn254;
use bn254::{ecadd_tests, ecmul_tests, ecpairing_tests};

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    ecadd_tests(&reth_crypto);
    ecmul_tests(&reth_crypto);
    ecpairing_tests(&reth_crypto); // TODO: It does not work with hints [Hints too large]
}

#![no_main]
ziskos::entrypoint!(main);

mod bls12_381;
use bls12_381::{
    bls12_381_g1_add_tests, bls12_381_g1_msm_tests, bls12_381_g1_mul_tests, bls12_381_g2_add_tests,
    bls12_381_g2_msm_tests, bls12_381_g2_mul_tests, bls12_381_map_fp2_to_g2_tests,
    bls12_381_map_fp_to_g1_tests, bls12_381_pairing_tests, bls12_381_point_evaluation_tests,
};

use guest_reth::CustomEvmCrypto;

fn main() {
    let reth_crypto = CustomEvmCrypto::default();

    bls12_381_g1_add_tests(&reth_crypto);
    bls12_381_g1_mul_tests(&reth_crypto);
    bls12_381_g1_msm_tests(&reth_crypto); // TODO: It does not work with hints [Hints too large]
    bls12_381_g2_add_tests(&reth_crypto);
    bls12_381_g2_mul_tests(&reth_crypto);
    bls12_381_g2_msm_tests(&reth_crypto); // TODO: It does not work with hints [Hints too large]
    bls12_381_map_fp_to_g1_tests(&reth_crypto);
    bls12_381_map_fp2_to_g2_tests(&reth_crypto);
    bls12_381_pairing_tests(&reth_crypto); // TODO: It does not work with hints [Hints too large]
    bls12_381_point_evaluation_tests(&reth_crypto);
}
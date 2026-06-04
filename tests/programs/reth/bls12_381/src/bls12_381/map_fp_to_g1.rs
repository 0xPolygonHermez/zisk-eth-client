use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g1_result, parse_fp_padded};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsMapFpToG1TestCase {
    pub name: String,
    pub fe: [u8; 48],
    pub expected: Option<[u8; 128]>,
}

fn parse_bls_map_fp_to_g1_test(test: &PrecompileTestCase) -> Result<BlsMapFpToG1TestCase, String> {
    if test.input.len() != 64 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let fe = parse_fp_padded(&test.input[0..64])?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 128];
            e.copy_from_slice(&bytes[..128]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsMapFpToG1TestCase {
        name: test.name.clone(),
        fe,
        expected,
    })
}

pub fn bls12_381_map_fp_to_g1_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsMapG1.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsMapG1.json"
    )));

    for test in &tests {
        match parse_bls_map_fp_to_g1_test(test) {
            Ok(t) => {
                let result = crypto.bls12_381_fp_to_g1(&t.fe);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "Map FP to G1 {} should succeed", t.name);
                        let padded = pad_g1_result(&result.unwrap());
                        assert_eq!(padded, expected, "Map FP to G1 {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "Map FP to G1 {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "Map FP to G1 {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 Map FP to G1 tests passed!");
}

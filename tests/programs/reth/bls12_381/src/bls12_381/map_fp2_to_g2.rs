use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g2_result, parse_fp_padded};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsMapFp2ToG2TestCase {
    pub name: String,
    pub fe0: [u8; 48],
    pub fe1: [u8; 48],
    pub expected: Option<[u8; 256]>,
}

fn parse_bls_map_fp2_to_g2_test(
    test: &PrecompileTestCase,
) -> Result<BlsMapFp2ToG2TestCase, String> {
    if test.input.len() != 128 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let fe0 = parse_fp_padded(&test.input[0..64])?;
    let fe1 = parse_fp_padded(&test.input[64..128])?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 256];
            e.copy_from_slice(&bytes[..256]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsMapFp2ToG2TestCase { name: test.name.clone(), fe0, fe1, expected })
}

pub fn bls12_381_map_fp2_to_g2_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsMapG2.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsMapG2.json"
    )));

    for test in &tests {
        match parse_bls_map_fp2_to_g2_test(test) {
            Ok(t) => {
                let result = crypto.bls12_381_fp2_to_g2((t.fe0, t.fe1));
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "Map FP2 to G2 {} should succeed", t.name);
                        let padded = pad_g2_result(&result.unwrap());
                        assert_eq!(padded, expected, "Map FP2 to G2 {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "Map FP2 to G2 {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "Map FP2 to G2 {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 Map FP2 to G2 tests passed!");
}

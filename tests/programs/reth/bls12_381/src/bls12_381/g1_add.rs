use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g1_result, parse_g1_point_padded, G1Point};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsG1AddTestCase {
    pub name: String,
    pub p1: G1Point,
    pub p2: G1Point,
    pub expected: Option<[u8; 128]>,
}

fn parse_bls_g1_add_test(test: &PrecompileTestCase) -> Result<BlsG1AddTestCase, String> {
    if test.input.len() != 256 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let p1 = parse_g1_point_padded(&test.input[..128])?;
    let p2 = parse_g1_point_padded(&test.input[128..256])?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 128];
            e.copy_from_slice(&bytes[..128]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG1AddTestCase { name: test.name.clone(), p1, p2, expected })
}

pub fn bls12_381_g1_add_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsG1Add.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG1Add.json"
    )));

    for test in &tests {
        match parse_bls_g1_add_test(test) {
            Ok(t) => {
                let result = crypto.bls12_381_g1_add(t.p1, t.p2);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G1 Add {} should succeed", t.name);
                        let padded = pad_g1_result(&result.unwrap());
                        assert_eq!(padded, expected, "G1 Add {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G1 Add {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G1 Add {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G1 Add tests passed!");
}

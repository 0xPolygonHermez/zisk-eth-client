use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g2_result, parse_g2_point_padded, G2Point};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsG2AddTestCase {
    pub name: String,
    pub p1: G2Point,
    pub p2: G2Point,
    pub expected: Option<[u8; 256]>,
}

fn parse_bls_g2_add_test(test: &PrecompileTestCase) -> Result<BlsG2AddTestCase, String> {
    if test.input.len() != 512 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let p1 = parse_g2_point_padded(&test.input[..256])?;
    let p2 = parse_g2_point_padded(&test.input[256..512])?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 256];
            e.copy_from_slice(&bytes[..256]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG2AddTestCase {
        name: test.name.clone(),
        p1,
        p2,
        expected,
    })
}

pub fn bls12_381_g2_add_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsG2Add.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG2Add.json"
    )));

    for test in &tests {
        match parse_bls_g2_add_test(test) {
            Ok(t) => {
                let result = crypto.bls12_381_g2_add(t.p1, t.p2);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G2 Add {} should succeed", t.name);
                        let padded = pad_g2_result(&result.unwrap());
                        assert_eq!(padded, expected, "G2 Add {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G2 Add {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G2 Add {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G2 Add tests passed!");
}

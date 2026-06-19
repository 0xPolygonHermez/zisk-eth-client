use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g2_result, parse_g2_point_padded, G2Point};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsG2MulTestCase {
    pub name: String,
    pub point: G2Point,
    pub scalar: [u8; 32],
    pub expected: Option<[u8; 256]>,
}

fn parse_bls_g2_mul_test(test: &PrecompileTestCase) -> Result<BlsG2MulTestCase, String> {
    if test.input.len() != 288 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let point = parse_g2_point_padded(&test.input[..256])?;
    let mut scalar = [0u8; 32];
    scalar.copy_from_slice(&test.input[256..288]);
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 256];
            e.copy_from_slice(&bytes[..256]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG2MulTestCase {
        name: test.name.clone(),
        point,
        scalar,
        expected,
    })
}

pub fn bls12_381_g2_mul_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsG2Mul.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG2Mul.json"
    )));

    for test in &tests {
        match parse_bls_g2_mul_test(test) {
            Ok(t) => {
                let pairs = vec![(t.point, t.scalar)];
                let mut iter = pairs.into_iter().map(Ok);
                let result = crypto.bls12_381_g2_msm(&mut iter);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G2 Mul {} should succeed", t.name);
                        let padded = pad_g2_result(&result.unwrap());
                        assert_eq!(padded, expected, "G2 Mul {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G2 Mul {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G2 Mul {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G2 Mul tests passed!");
}

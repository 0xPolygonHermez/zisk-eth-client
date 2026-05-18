use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g1_result, parse_g1_point_padded, G1Point};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

pub struct BlsG1MulTestCase {
    pub name: String,
    pub point: G1Point,
    pub scalar: [u8; 32],
    pub expected: Option<[u8; 128]>,
}

pub fn parse_bls_g1_mul_test(test: &PrecompileTestCase) -> Result<BlsG1MulTestCase, String> {
    if test.input.len() != 160 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let point = parse_g1_point_padded(&test.input[..128])?;
    let mut scalar = [0u8; 32];
    scalar.copy_from_slice(&test.input[128..160]);
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 128];
            e.copy_from_slice(&bytes[..128]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG1MulTestCase { name: test.name.clone(), point, scalar, expected })
}

pub fn bls12_381_g1_mul_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsG1Mul.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG1Mul.json"
    )));

    for test in &tests {
        match parse_bls_g1_mul_test(test) {
            Ok(t) => {
                let pairs = vec![(t.point, t.scalar)];
                let mut iter = pairs.into_iter().map(Ok);
                let result = crypto.bls12_381_g1_msm(&mut iter);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G1 Mul {} should succeed", t.name);
                        let padded = pad_g1_result(&result.unwrap());
                        assert_eq!(padded, expected, "G1 Mul {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G1 Mul {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G1 Mul {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G1 Mul tests passed!");
}

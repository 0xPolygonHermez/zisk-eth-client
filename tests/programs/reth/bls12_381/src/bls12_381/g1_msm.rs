use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g1_result, parse_g1_point_padded, G1PointScalar};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

pub struct BlsG1MsmTestCase {
    pub name: String,
    pub pairs: Vec<G1PointScalar>,
    pub expected: Option<[u8; 128]>,
}

fn parse_g1_msm_pairs(input: &[u8]) -> Result<Vec<G1PointScalar>, String> {
    let mut pairs = Vec::new();
    for i in 0..input.len() / 160 {
        let o = i * 160;
        let point = parse_g1_point_padded(&input[o..o + 128])?;
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(&input[o + 128..o + 160]);
        pairs.push((point, scalar));
    }
    Ok(pairs)
}

fn parse_bls_g1_msm_test(test: &PrecompileTestCase) -> Result<BlsG1MsmTestCase, String> {
    if test.input.is_empty() || test.input.len() % 160 != 0 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let pairs = parse_g1_msm_pairs(&test.input)?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 128];
            e.copy_from_slice(&bytes[..128]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG1MsmTestCase { name: test.name.clone(), pairs, expected })
}

pub fn bls12_381_g1_msm_tests(crypto: &CustomEvmCrypto) {
    let mut tests =
        parse_precompile_json(include_str!("../testdata/blsG1MultiExp.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG1MultiExp.json"
    )));

    for test in &tests {
        match parse_bls_g1_msm_test(test) {
            Ok(t) => {
                let mut iter = t.pairs.into_iter().map(Ok);
                let result = crypto.bls12_381_g1_msm(&mut iter);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G1 MSM {} should succeed", t.name);
                        let padded = pad_g1_result(&result.unwrap());
                        assert_eq!(padded, expected, "G1 MSM {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G1 MSM {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G1 MSM {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G1 MSM tests passed!");
}

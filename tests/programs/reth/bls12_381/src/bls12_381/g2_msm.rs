use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{pad_g2_result, parse_g2_point_padded, G2PointScalar};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsG2MsmTestCase {
    pub name: String,
    pub pairs: Vec<G2PointScalar>,
    pub expected: Option<[u8; 256]>,
}

fn parse_g2_msm_pairs(input: &[u8]) -> Result<Vec<G2PointScalar>, String> {
    let mut pairs = Vec::new();
    for i in 0..input.len() / 288 {
        let o = i * 288;
        let point = parse_g2_point_padded(&input[o..o + 256])?;
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(&input[o + 256..o + 288]);
        pairs.push((point, scalar));
    }
    Ok(pairs)
}

fn parse_bls_g2_msm_test(test: &PrecompileTestCase) -> Result<BlsG2MsmTestCase, String> {
    if test.input.is_empty() || test.input.len() % 288 != 0 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let pairs = parse_g2_msm_pairs(&test.input)?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            let mut e = [0u8; 256];
            e.copy_from_slice(&bytes[..256]);
            Some(e)
        }
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsG2MsmTestCase {
        name: test.name.clone(),
        pairs,
        expected,
    })
}

pub fn bls12_381_g2_msm_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsG2MultiExp.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsG2MultiExp.json"
    )));

    // The `jwasinger` cases scale up to 1024 pairs and dominate runtime
    // (~94% of total G2 MSM work). Skipped by default.
    if !cfg!(feature = "heavy-tests") {
        tests.retain(|t| !t.name.contains("jwasinger"));
    }

    for test in &tests {
        match parse_bls_g2_msm_test(test) {
            Ok(t) => {
                let mut iter = t.pairs.into_iter().map(Ok);
                let result = crypto.bls12_381_g2_msm(&mut iter);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "G2 MSM {} should succeed", t.name);
                        let padded = pad_g2_result(&result.unwrap());
                        assert_eq!(padded, expected, "G2 MSM {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "G2 MSM {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "G2 MSM {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 G2 MSM tests passed!");
}

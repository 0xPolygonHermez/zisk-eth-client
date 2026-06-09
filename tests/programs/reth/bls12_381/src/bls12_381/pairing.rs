use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{parse_g1_point_padded, parse_g2_point_padded, G1Point, G2Point};
use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct BlsPairingTestCase {
    pub name: String,
    pub pairs: Vec<(G1Point, G2Point)>,
    pub expected: Option<bool>,
}

fn parse_pairing_pairs(input: &[u8]) -> Result<Vec<(G1Point, G2Point)>, String> {
    let mut pairs = Vec::new();
    for i in 0..input.len() / 384 {
        let o = i * 384;
        let g1 = parse_g1_point_padded(&input[o..o + 128])?;
        let g2 = parse_g2_point_padded(&input[o + 128..o + 384])?;
        pairs.push((g1, g2));
    }
    Ok(pairs)
}

fn parse_bls_pairing_test(test: &PrecompileTestCase) -> Result<BlsPairingTestCase, String> {
    if test.input.is_empty() || test.input.len() % 384 != 0 {
        return Err(format!("invalid input length: {}", test.input.len()));
    }

    let pairs = parse_pairing_pairs(&test.input)?;
    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => Some(bytes.len() == 32 && bytes[31] == 1),
        ExpectedOutcome::Failure(_) => None,
    };
    Ok(BlsPairingTestCase {
        name: test.name.clone(),
        pairs,
        expected,
    })
}

pub fn bls12_381_pairing_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blsPairing.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blsPairing.json"
    )));

    for test in &tests {
        match parse_bls_pairing_test(test) {
            Ok(t) => {
                let result = crypto.bls12_381_pairing_check(&t.pairs);
                match t.expected {
                    Some(expected) => {
                        assert!(result.is_ok(), "Pairing {} should succeed", t.name);
                        assert_eq!(result.unwrap(), expected, "Pairing {} mismatch", t.name);
                    }
                    None => {
                        assert!(result.is_err(), "Pairing {} should fail", t.name);
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "Pairing {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All BLS12-381 Pairing tests passed!");
}

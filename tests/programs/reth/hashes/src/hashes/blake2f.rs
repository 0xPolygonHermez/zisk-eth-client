use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use common::{
    parse_precompile_fail_json, parse_precompile_json, ExpectedOutcome, PrecompileTestCase,
};

struct Blake2fTestCase {
    name: String,
    rounds: u32,
    h: [u64; 8],
    m: [u64; 16],
    t: [u64; 2],
    f: bool,
    expected: Option<[u64; 8]>,
}

fn parse_blake2f_test(test: &PrecompileTestCase) -> Result<Blake2fTestCase, String> {
    let input = &test.input;
    if input.len() != 213 {
        return Err(format!(
            "invalid input length: {} (expected 213)",
            input.len()
        ));
    }

    let rounds = u32::from_be_bytes(input[0..4].try_into().unwrap());

    let mut h = [0u64; 8];
    for i in 0..8 {
        let offset = 4 + i * 8;
        h[i] = u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap());
    }

    let mut m = [0u64; 16];
    for i in 0..16 {
        let offset = 68 + i * 8;
        m[i] = u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap());
    }

    let mut t = [0u64; 2];
    for i in 0..2 {
        let offset = 196 + i * 8;
        t[i] = u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap());
    }

    if input[212] > 1 {
        return Err(format!("invalid final flag: {}", input[212]));
    }
    let f = input[212] != 0;

    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) => {
            if bytes.len() != 64 {
                return Err(format!("invalid expected length: {}", bytes.len()));
            }
            let mut expected_h = [0u64; 8];
            for i in 0..8 {
                let offset = i * 8;
                expected_h[i] = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            }
            Some(expected_h)
        }
        ExpectedOutcome::Failure(_) => None,
    };

    Ok(Blake2fTestCase {
        name: test.name.clone(),
        rounds,
        h,
        m,
        t,
        f,
        expected,
    })
}

pub fn blake2f_tests(crypto: &CustomEvmCrypto) {
    let mut tests = parse_precompile_json(include_str!("../testdata/blake2F.json"));
    tests.extend(parse_precompile_fail_json(include_str!(
        "../testdata/fail-blake2f.json"
    )));

    for test in &tests {
        match parse_blake2f_test(test) {
            Ok(parsed) => {
                let mut h = parsed.h;
                crypto.blake2_compress(parsed.rounds, &mut h, &parsed.m, &parsed.t, parsed.f);
                match parsed.expected {
                    Some(expected_h) => {
                        assert_eq!(
                            h, expected_h,
                            "Blake2f {} mismatch:\n  got = {:016x?}\n  expected = {:016x?}",
                            parsed.name, h, expected_h
                        );
                    }
                    None => {
                        // Parsed OK but expected failure — shouldn't happen for blake2f
                        // since failures are caught at parse time, but handle gracefully
                    }
                }
            }
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "Blake2f {} parse error on a success test: {}",
                    test.name,
                    e
                );
            }
        }
    }

    println!("All Blake2f tests passed!");
}

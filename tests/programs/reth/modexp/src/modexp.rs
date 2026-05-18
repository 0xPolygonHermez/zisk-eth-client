use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use common::{hex_to_vec, parse_precompile_json, PrecompileTestCase};

struct ModexpTestCase {
    name: String,
    base: Vec<u8>,
    exp: Vec<u8>,
    modulus: Vec<u8>,
    expected: Vec<u8>,
}

fn be_bytes_to_usize(bytes: &[u8]) -> usize {
    let mut val: usize = 0;
    for &b in bytes {
        val = (val << 8) | (b as usize);
    }
    val
}

fn parse_modexp_test(test: &PrecompileTestCase) -> Option<ModexpTestCase> {
    if test.input.len() < 96 {
        return None;
    }
    let base_len = be_bytes_to_usize(&test.input[0..32]);
    let exp_len = be_bytes_to_usize(&test.input[32..64]);
    let mod_len = be_bytes_to_usize(&test.input[64..96]);
    let data = &test.input[96..];
    if data.len() < base_len + exp_len + mod_len {
        return None;
    }
    Some(ModexpTestCase {
        name: test.name.clone(),
        base: data[..base_len].to_vec(),
        exp: data[base_len..base_len + exp_len].to_vec(),
        modulus: data[base_len + exp_len..base_len + exp_len + mod_len].to_vec(),
        expected: test.expected.unwrap_success().clone(),
    })
}

pub fn modexp_tests(crypto: &CustomEvmCrypto) {
    modexp_early_return_tests(crypto);
    modexp_256bit_tests(crypto);
    modexp_512bit_tests(crypto);
    modexp_4096bit_tests(crypto);
    modexp_json_tests(crypto);
    println!("All Modexp tests passed!");
}

fn modexp_json_tests(crypto: &CustomEvmCrypto) {
    let json_files: &[(&str, &str)] = &[
        ("modexp_eip198", include_str!("../testdata/modexp.json")),
        ("modexp_eip2565", include_str!("../testdata/modexp_eip2565.json")),
        ("modexp_eip7883", include_str!("../testdata/modexp_eip7883.json")),
    ];

    for (file_name, json_content) in json_files {
        let tests = parse_precompile_json(json_content);
        for test in &tests {
            let Some(parsed) = parse_modexp_test(test) else { continue };
            let result = crypto.modexp(&parsed.base, &parsed.exp, &parsed.modulus);
            assert!(result.is_ok(), "Modexp {} ({}) should succeed", parsed.name, file_name);
            let result = result.unwrap();
            assert_eq!(result, parsed.expected, "Modexp {} ({}) mismatch", parsed.name, file_name);
        }
    }
}

fn modexp_early_return_tests(crypto: &CustomEvmCrypto) {
    // M == 1 should return 0
    // base=0, exp=0, mod=1 -> 0
    let base = hex_to_vec("00");
    let exp = hex_to_vec("00");
    let modulus = hex_to_vec("01");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "0^0 mod 1 should be 0");

    // base=1, exp=1, mod=1 -> 0
    let base = hex_to_vec("01");
    let exp = hex_to_vec("01");
    let modulus = hex_to_vec("01");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "1^1 mod 1 should be 0");

    // E == 0, M > 1 should return 1
    // base=0, exp=0, mod=2 -> 1
    let base = hex_to_vec("00");
    let exp = hex_to_vec("00");
    let modulus = hex_to_vec("02");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("01"), "0^0 mod 2 should be 1");

    // base=1, exp=0, mod=2 -> 1
    let base = hex_to_vec("01");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("01"), "1^0 mod 2 should be 1");

    // B == 0, E > 0, M > 1 should return 0
    let base = hex_to_vec("00");
    let exp = hex_to_vec("01");
    let modulus = hex_to_vec("02");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "0^1 mod 2 should be 0");

    // B == 1, E > 0, M > 1 should return 1
    let base = hex_to_vec("01");
    let exp = hex_to_vec("01");
    let modulus = hex_to_vec("02");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("01"), "1^1 mod 2 should be 1");
}

fn modexp_256bit_tests(crypto: &CustomEvmCrypto) {
    // b == k·m (at any point of the exponentiations) should return 0
    // 4^78 mod 4 = 0
    let base = hex_to_vec("04");
    let exp = hex_to_vec("4e"); // 78
    let modulus = hex_to_vec("04");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "4^78 mod 4 should be 0");

    // 16^78 mod 4 = 0
    let base = hex_to_vec("10");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "16^78 mod 4 should be 0");

    // 2^2 mod 4 = 0
    let base = hex_to_vec("02");
    let exp = hex_to_vec("02");
    let modulus = hex_to_vec("04");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("00"), "2^2 mod 4 should be 0");

    // Simple test: 3^5 mod 7 = 243 mod 7 = 5
    let base = hex_to_vec("03");
    let exp = hex_to_vec("05");
    let modulus = hex_to_vec("07");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("05"), "3^5 mod 7 should be 5");

    // 2^10 mod 1000 = 1024 mod 1000 = 24 = 0x18
    let base = hex_to_vec("02");
    let exp = hex_to_vec("0a");
    let modulus = hex_to_vec("03e8"); // 1000
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("0018"), "2^10 mod 1000 should be 24");

    // 256-bit modexp from EIP-198 example
    // base = 3
    // exp = 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2e (secp256k1 order - 1)
    // mod = 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f (secp256k1 prime)
    // result should be 1 (Fermat's little theorem)
    let base = hex_to_vec("03");
    let exp = hex_to_vec("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2e");
    let modulus = hex_to_vec("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(
        result,
        hex_to_vec("0000000000000000000000000000000000000000000000000000000000000001"),
        "Fermat's little theorem test"
    );

    // BN254 scalar field test
    // base = 0x7aa27b83e565bec0e483a9ec581780eb12d1 (fits in base)
    // exp = r - 1 = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffffff
    // mod = r = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
    let base = hex_to_vec("000000000000000000000000000000007aa27b83e565bec0e483a9ec581780eb12d1");
    let exp = hex_to_vec("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffffff");
    let modulus = hex_to_vec("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(
        result,
        hex_to_vec("285a542908e6755b8bbc45af189aec21aaaa60c16c4dc2bf43e9228f869d68ae"),
        "BN254 scalar field modexp"
    );

    // worst case for 256-bit: 3^(p-1) mod p where p is largest 256-bit prime
    let base = hex_to_vec("03");
    let exp = hex_to_vec("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2e");
    let modulus = hex_to_vec("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(
        result,
        hex_to_vec("0000000000000000000000000000000000000000000000000000000000000001"),
        "worst case 256-bit"
    );
}

fn modexp_512bit_tests(crypto: &CustomEvmCrypto) {
    // 512-bit exponent tests

    // Simple 512-bit test: 2^(2^256 + 3) mod 7
    let base = hex_to_vec("02");
    let exp = hex_to_vec("00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003");
    let modulus = hex_to_vec("07");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result, hex_to_vec("02"), "2^(2^256 + 3) mod 7 should be 2");

    // 3^65537 mod (2^512 - 1) - RSA-like exponent
    let base = hex_to_vec("03");
    let exp = hex_to_vec("010001"); // 65537
    let modulus = hex_to_vec("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    let result = crypto.modexp(&base, &exp, &modulus).unwrap();
    assert_eq!(result.len(), 64, "512-bit modulus should give 64-byte result");
}

fn modexp_4096bit_tests(crypto: &CustomEvmCrypto) {
    // RSA-2048 style: base^65537 mod n (where n is 2048 bits = 256 bytes)
    // Using a simple modulus for testing

    // Small base, RSA exponent, medium modulus
    let base = hex_to_vec("1234567890abcdef");
    let exp = hex_to_vec("010001"); // 65537
    let modulus = hex_to_vec(
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc90fdaa22168c234c4c6628b80dc1cd129024e088a67cc74020bbea63b139b22514a08798e3404ddef9519b3cd3a431b302b0a6df25f14374fe1356d6d51c245e485b576625e7ec6f44c42e9a637ed6b0bff5cb6f406b7edee386bfb5a899fa5ae9f24117c4b1fe649286651ece45b3dc2007cb8a163bf0598da48361c55d39a69163fa8fd24cf5f83655d23dca3ad961c62f356208552bb9ed529077096966d670c354e4abc9804f1746c08ca18217c32905e462e36ce3be39e772c180e86039b2783a2ec07a28fb5c55df06f4c52c9de2bcbf6955817183995497cea956ae515d2261898fa051015728e5a8aacaa68ffffffffffffffff"
    );
    let result = crypto.modexp(&base, &exp, &modulus);
    assert!(result.is_ok(), "4096-bit modexp should succeed");
    assert_eq!(result.unwrap().len(), modulus.len(), "result length should match modulus length");
}

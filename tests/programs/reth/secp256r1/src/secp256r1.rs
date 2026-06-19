use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use common::{parse_precompile_json, ExpectedOutcome, PrecompileTestCase};

struct P256VerifyTestCase {
    name: String,
    msg: [u8; 32],
    sig: [u8; 64],
    pk: [u8; 64],
    expected: bool,
}

fn parse_p256_verify_test(test: &PrecompileTestCase) -> P256VerifyTestCase {
    let mut input = test.input.clone();
    input.resize(160, 0);

    let mut msg = [0u8; 32];
    msg.copy_from_slice(&input[0..32]);

    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&input[32..64]);
    sig[32..].copy_from_slice(&input[64..96]);

    let mut pk = [0u8; 64];
    pk[..32].copy_from_slice(&input[96..128]);
    pk[32..].copy_from_slice(&input[128..160]);

    let expected = matches!(test.expected, ExpectedOutcome::Success(ref b) if !b.is_empty());

    P256VerifyTestCase {
        name: test.name.clone(),
        msg,
        sig,
        pk,
        expected,
    }
}

pub fn p256_verify_tests(crypto: &CustomEvmCrypto) {
    let tests = parse_precompile_json(include_str!("../testdata/p256Verify.json"));
    for test in &tests {
        let t = parse_p256_verify_test(test);
        let result = crypto.secp256r1_verify_signature(&t.msg, &t.sig, &t.pk);
        assert_eq!(
            result, t.expected,
            "p256Verify {} mismatch: got {}, expected {}",
            t.name, result, t.expected
        );
    }
}

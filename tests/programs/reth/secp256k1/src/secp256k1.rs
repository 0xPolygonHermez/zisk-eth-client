use alloy_consensus::crypto::CryptoProvider;
use alloy_primitives::Address;

use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use common::{parse_precompile_json, ExpectedOutcome, PrecompileTestCase};

struct EcRecoverTestCase {
    name: String,
    hash: [u8; 32],
    v: u8,
    r: [u8; 32],
    s: [u8; 32],
    expected: Option<[u8; 32]>,
}

fn parse_ecrecover_test(test: &PrecompileTestCase) -> Result<EcRecoverTestCase, String> {
    let mut input = test.input.clone();
    input.resize(128, 0);

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&input[0..32]);

    if !(input[32..63].iter().all(|&b| b == 0) && matches!(input[63], 27 | 28)) {
        return Err(format!("Invalid v in test {}", test.name));
    }
    let v = input[63];

    let mut r = [0u8; 32];
    r.copy_from_slice(&input[64..96]);

    let mut s = [0u8; 32];
    s.copy_from_slice(&input[96..128]);

    let expected = match &test.expected {
        ExpectedOutcome::Success(bytes) if bytes.is_empty() => None,
        ExpectedOutcome::Success(bytes) => {
            let mut addr = [0u8; 32];
            let len = bytes.len().min(32);
            addr[..len].copy_from_slice(&bytes[..len]);
            Some(addr)
        }
        ExpectedOutcome::Failure(_) => None,
    };

    Ok(EcRecoverTestCase { name: test.name.clone(), hash, v, r, s, expected })
}

/// Helper to convert v (27 or 28) to recid (0 or 1)
fn v_to_recid(v: u8) -> u8 {
    v - 27
}

/// Helper to build signature bytes from r and s (big-endian 32-byte each)
fn build_sig(r: [u8; 32], s: [u8; 32]) -> [u8; 64] {
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&r);
    sig[32..].copy_from_slice(&s);
    sig
}

/// Helper to build 65-byte signature (r || s || v) for recover_signer_unchecked
fn build_sig_65(r: [u8; 32], s: [u8; 32], v: u8) -> [u8; 65] {
    let mut sig = [0u8; 65];
    sig[..32].copy_from_slice(&r);
    sig[32..64].copy_from_slice(&s);
    sig[64] = v_to_recid(v);
    sig
}

/// Convert hex string (without 0x prefix) to 32-byte array
fn hex_to_32(hex: &str) -> [u8; 32] {
    let bytes = hex::decode(hex).expect("valid hex");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

/// Convert hex string (without 0x prefix) to 20-byte Address
fn hex_to_address(hex: &str) -> Address {
    let bytes = hex::decode(hex).expect("valid hex");
    Address::from_slice(&bytes)
}

// ============================================================
// ecrecover_tx tests (using recover_signer_unchecked)
// These allow low S values only (s < N/2)
// ============================================================
pub fn ecrecover_tx_tests(crypto: &CustomEvmCrypto) {
    /////////
    // Valid tests
    //////////
    let hash = hex_to_32("d9eba16ed0ecae432b71fe008c98cc872bb4cc214d3220a36f365326cf807d68");
    let r = hex_to_32("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"); // ECGX
    let s = hex_to_32("265e99e47ad31bb2cab9646c504576b3abc6939a1710afc08cbf3034d73214b8");
    let v = 0x1c;
    let expected = hex_to_address("BC44674AD5868F642EAD3FDF94E2D9C9185EAFB7");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #100a failed");

    let hash = hex_to_32("d9eba16ed0ecae432b71fe008c98cc872bb4cc214d3220a36f365326cf807d68");
    let r = hex_to_32("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"); // ECGX
    let s = hex_to_32("265e99e47ad31bb2cab9646c504576b3abc6939a1710afc08cbf3034d73214b8");
    let v = 0x1b;
    let expected = hex_to_address("EE3FEFB38D4E5C7337818F635DEE7609F67CFDB8");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #100b failed");

    // #100 first valid ecrecover_tx
    let hash = hex_to_32("d9eba16ed0ecae432b71fe008c98cc872bb4cc214d3220a36f365326cf807d68");
    let r = hex_to_32("ddd0a7290af9526056b4e35a077b9a11b513aa0028ec6c9880948544508f3c63");
    let s = hex_to_32("265e99e47ad31bb2cab9646c504576b3abc6939a1710afc08cbf3034d73214b8");
    let v = 0x1c;
    let expected = hex_to_address("14791697260e4c9a71f18484c9f997b308e59325");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #100c failed");

    // #0 valid ecrecover_tx
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eebe");
    let r = hex_to_32("7dff8b06f4914ff0be0e02edf967ce8f13224cb4819e3833b777867db61f8a62");
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c4");
    let v = 0x1b;
    let expected = hex_to_address("bec80D04A24CD4D811876fF40F31260C339d63C2");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #0 failed");

    // #1 valid ecrecover_tx
    let hash = hex_to_32("ee43d51baa54831bfd9b03c4b17b59a594378d777e418020c3358f1822cec07d");
    let r = hex_to_32("ea678b4b3ebde1e877401b1bf67fd59e17c0eb75e6da8ecd9cc38620099e0c65");
    let s = hex_to_32("2b954333cfe2b4bf97c4abbcede04a5edda0688b9e965a4bff3d6a45bed2a6e8");
    let v = 0x1b;
    let expected = hex_to_address("d571a2180a8647e77adfc109C49bd2137c7a71b5");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #1 failed");

    // #2 valid ecrecover_tx
    let hash = hex_to_32("fd98b0bfc9eecc813b958263c08db17c932bfe1c2ae7c21a12bb42860d9d9c1f");
    let r = hex_to_32("86b2f6d4bae0e1e1139bc29378fd243d33de35822652144b0b04af346cbf3ad2");
    let s = hex_to_32("5ec4a8672d44b21bdc2203ab8adab6ae53d4c57a2d832090cab710f26600e2fc");
    let v = 0x1b;
    let expected = hex_to_address("52855436E41c2671759d54103e71A3d5Fe27439C");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #2 failed");

    // #3 valid ecrecover_tx
    let hash = hex_to_32("c401c7d7baa568f5928028def2818ec62b70db9b738ab3f4d36326dac17839e5");
    let r = hex_to_32("4a95890f1102c6a6684815f4274d68d64d6deb59e5d524d1a6394256dd188b33");
    let s = hex_to_32("3e53619fe2f4c67ac8ff4103da0740fd7c912be5b176329a74764bd790e42f51");
    let v = 0x1b;
    let expected = hex_to_address("A4E24c3ea459D50409dd31Cf1C35A222ADA889fa");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #3 failed");

    // #4 valid ecrecover_tx
    let hash = hex_to_32("60049b234e7fd86ba5354613066d0d67ccdd993d5322656c56562d48c208002a");
    let r = hex_to_32("f06fbfb021b185a599ec35b0c4a85f5c329abee4f91c4027cb90210c66868e99");
    let s = hex_to_32("1e0e24cab3e119fea57b012b25bfd65f710fe567a53d1a414208f0385f37be1c");
    let v = 0x1b;
    let expected = hex_to_address("2302Fdd6dB52D32b84cc7648Fb9d8978a005E382");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #4 failed");

    // #5 valid ecrecover_tx
    let hash = hex_to_32("fda892e54ef8d0fc5d34f3c78fd9262ebc97c448eec72348340c902ff8f82d86");
    let r = hex_to_32("e9713e52d8ca16c7723ea73ff89244d1c5bfe6611f56d8a2cebcf25f546c0bb3");
    let s = hex_to_32("627566979c5b6e3a5b8e06ba56f22eed367a9b54ed9c3c54d01a29012435fe02");
    let v = 0x1c;
    let expected = hex_to_address("Cc665b2CaA43684c99f0EB59751DC1a8C4dBd9Ee");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #5 failed");

    // #6 valid ecrecover_tx
    let hash = hex_to_32("1df5d6cd09999848b734ef91a35a31840b6f4f7bc42fa4005ef44a3a0da75fd7");
    let r = hex_to_32("8c661a2e0ae1ffe716d34e6d1ceacfd75326da91bf3af225ed675602a00ebf21");
    let s = hex_to_32("3de21452fd054750ae68326d7781ed903b278bb1ac1f52fb8f55e8a4c9954000");
    let v = 0x1b;
    let expected = hex_to_address("1F34358a423C2FED839090Dd943A728Dbd711e62");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #6 failed");

    // #7 valid ecrecover_tx
    let hash = hex_to_32("84254d72d3a17a61a1ff7dbef2cbb063f2913a407a15ee6012168ce0ce5bc500");
    let r = hex_to_32("4937d15fdde73a18520a70b8e8f6fb05e9c172d8b04b02c0246cd5c9fa76ff97");
    let s = hex_to_32("70ac180d31a5336bc902e317c6b5850b151ea7a2cff0595c61d67bf49d1fc210");
    let v = 0x1b;
    let expected = hex_to_address("0C470e0895Bbe22A5cFeD06002e29e8ADCEB59E1");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #7 failed");

    // #8 valid ecrecover_tx
    let hash = hex_to_32("cbfb5477b23d3f14e821799805060a4be3dc7ec51f007ac78b810941a5693723");
    let r = hex_to_32("465e8a566e29f97b29980b458bff07cdeff5e020f38d541c3a980eb4642b4606");
    let s = hex_to_32("531eaeeeec12998af811c435a79a737d3dbb7411ed4d937080b6201a46703bf0");
    let v = 0x1b;
    let expected = hex_to_address("6777D222b69b69F41ee3DBFCf6baee332999f347");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #8 failed");

    // #9 valid ecrecover_tx
    let hash = hex_to_32("f27fb8414a5aaed947f91ff458ea71f6eed0b8949d3da51ad7325047aba2e081");
    let r = hex_to_32("493b8ca6e09c06450508b8eae88bf8103548b71135e1d8721b935a8822f28890");
    let s = hex_to_32("4dea2362974bb205c48e92bd88761b454f2f4e085dfb9beb0706588b6eb852a8");
    let v = 0x1c;
    let expected = hex_to_address("1f56A9F3Ca1F284978478D9363D2b345B23B198B");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #9 failed");

    // #10 valid ecrecover_tx
    let hash = hex_to_32("7d7e073c17eb159ad8372f82f9fa700a77a8a09dbfa7ccc2561504993be9ccca");
    let r = hex_to_32("8c23081e8211029ca641fe88235387a54d875b2fd64df1dbe87fb438316db708");
    let s = hex_to_32("41c8e2f71966a6553c03d9477145366764399a6ad23c2efdabbdf7004d21a7b7");
    let v = 0x1b;
    let expected = hex_to_address("6a7C3804E52D9B089767eA5D26232F9096F7B50e");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #10 failed");

    // #11 valid ecrecover_tx
    let hash = hex_to_32("8aec5831f7b02d4f714878d3729975afd5435e61e738a119ddca2e9777e04183");
    let r = hex_to_32("49b0018d05b6f36d461339633082c8a032543c4f4c5ae6af56ff5e48fd943372");
    let s = hex_to_32("34f096b3ee06d00f330697bddbe5e25d9207e5ccac214eb94ce96ed4507393c1");
    let v = 0x1b;
    let expected = hex_to_address("dD6F178Fa6ed5859154301C686C3cd12192CcC0A");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #11 failed");

    // #12 valid ecrecover_tx
    let hash = hex_to_32("c2c0f5c01b3e244bd9f40da627cfefbc45c53e76f26798cc195de61e3438585f");
    let r = hex_to_32("4a06e1f0012017c68294911be669e953e01c41f97b31e4f6830af4200d4bf253");
    let s = hex_to_32("2eafb33fbb92b512175812972ce89e54e50f28ecf27a051b4f2291da2cf0e1f3");
    let v = 0x1c;
    let expected = hex_to_address("5fFd05d8565FA51BB1aE1E4c7b980e1C91d2c939");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #12 failed");

    // #13 valid ecrecover_tx
    let hash = hex_to_32("1430fc999d7bfa8954983e43f80a27a07eb4845037bc344fc128b453f76877a3");
    let r = hex_to_32("88843aea48fff6790cb6f9657e1650e1386aa73a6bed50e8db2467d06445bed8");
    let s = hex_to_32("779ce085139a5fc49f33431c9d4794a16732d497158570690662ed779fb22ead");
    let v = 0x1c;
    let expected = hex_to_address("F329e07AFade0Fb45a599993dF4313DFaB5f80A6");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #13 failed");

    // #14 valid ecrecover_tx
    let hash = hex_to_32("8bc6619ef176973b22e4a0f0f81a6cd5badf091e452f910de38162b18c2f2ff6");
    let r = hex_to_32("347faebc1fc4c014692ffc7284fa28596d1494c29f29cdc52fa1c8d57ded6a5d");
    let s = hex_to_32("597a18da2e6c3c1eaa56d9611f50bf3d161a05e542afd29cf416b80f82ee5ef9");
    let v = 0x1c;
    let expected = hex_to_address("411559B990Fd50F1DF20B4D3780C4185d113A9Ee");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #14 failed");

    // #15 valid ecrecover_tx
    let hash = hex_to_32("b3dc90532569a73ab8fc3d41d6df176833133be5a697f076392f51001c715006");
    let r = hex_to_32("af4592a0ec76c84bb587b1bdd3b77b1207f7fd572e118f4278a80ec7c89666b9");
    let s = hex_to_32("0cb4bd62edd2d286b883d32006b799532737b5b90ed4a03ce0a6868a2f10edb6");
    let v = 0x1b;
    let expected = hex_to_address("B2E1a1FbfdAE0743f4539166309B72BB737716b1");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #15 failed");

    // #16 valid ecrecover_tx
    let hash = hex_to_32("c7367ffedb65fe022fa06a8c2855c69547cca19ee828d6615e5f7decafccbdc8");
    let r = hex_to_32("b7fd2fa91dd0ba816b37f4ad9b507ad3b32ec2a445544fabb0e231c5aeeefb18");
    let s = hex_to_32("244c004f8b78dc3a3c3db9bda9b39d6e6b11b9397baaf29d0529b7bac9490742");
    let v = 0x1c;
    let expected = hex_to_address("95DdaFE17e5550FF8ac7b2EF9D365d067aa7Ae91");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #16 failed");

    // #17 valid ecrecover_tx
    let hash = hex_to_32("898a015948f4e12fe71bb3f6015410f33d0b67842b551599762b51fecad5577c");
    let r = hex_to_32("a0d427856db72164abc59666e34cd4accb689f1800f71384d797a729c003598b");
    let s = hex_to_32("514d41eca750ebe03d75ed7182d81168af198760e3ac5077c5d88d1f85656039");
    let v = 0x1c;
    let expected = hex_to_address("9D39Cfa3F2e98CA109c45F28735436798CBBDEF8");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #17 failed");

    // #18 valid ecrecover_tx
    let hash = hex_to_32("add47d80decfc50037d37430ccdda08b3d39831e5355e99bfe5e08e00531182e");
    let r = hex_to_32("f74a32e691a8e0ee1b55fa4aeeebfb3eed091108ce03f7ef449846044f228d4a");
    let s = hex_to_32("308af50fef22673ae4a83c5e80c4f316829490b57a561c9f45a1570286d94e41");
    let v = 0x1b;
    let expected = hex_to_address("4AB0c3d24dfF7E98e16Edd61269997D089E97c9f");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #18 failed");

    // #19 valid ecrecover_tx
    let hash = hex_to_32("8fc27577b5290a3ab0d3407969123c719c475a28e687d943f88ac5abadf83420");
    let r = hex_to_32("8b9d9e9c4201b9b2d351b5dde35c0ebefa5446c2dbdd5e30dca02af69d8c29bd");
    let s = hex_to_32("047fb2ba1e4cc0a059ee64aae40d4ba48e6d3265426f602be275d286e45917f6");
    let v = 0x1b;
    let expected = hex_to_address("d02c6aAB18d3f40AA994A5B7F3c2be14B34EAB6e");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #19 failed");

    // #20 mess == change 1 bit
    let hash = hex_to_32("256e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("34E325D8023eb901c39747338C587b098fB75dF4");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #20 failed");

    // #21 mess == 0
    let hash = hex_to_32("0000000000000000000000000000000000000000000000000000000000000000");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("2a558C4cD662E0b74E289d746AEA2f8cf8e54f7c");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #21 failed");

    // #22 mess == field (N)
    let hash = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("2a558C4cD662E0b74E289d746AEA2f8cf8e54f7c");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #22 failed");

    // #23 mess == field - 1
    let hash = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("c41ABa9e06fac6976618820d04D247FfD38f62FF");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #23 failed");

    // #24 mess == field + 1
    let hash = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364142");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("7a343F50dd8fAFa76406F6Ee4dA1796FF9A06109");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #24 failed");

    // #25 mess == 1
    let hash = hex_to_32("0000000000000000000000000000000000000000000000000000000000000001");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("7a343F50dd8fAFa76406F6Ee4dA1796FF9A06109");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #25 failed");

    // #26 flip v: 28 --> 27. Valid ecrecover_tx
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1b;
    let expected = hex_to_address("E077fd3C958303e36309B9EE20AE9D3D59817232");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #26 failed");

    ////////
    // Invalid tests
    ////////
    // #29 r == 0 - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("0000000000000000000000000000000000000000000000000000000000000000");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #29 should fail (r == 0)");

    // #30 r == field (N) - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #30 should fail (r == N)");

    // #31 r > field - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364142");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #31 should fail (r > N)");

    // #32 r = field - 1 - should fail (r must be < N)
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #32 should fail (r == N-1, recovery fails)");

    // #33 s == 0 - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("0000000000000000000000000000000000000000000000000000000000000000");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #33 should fail (s == 0)");

    // #35 s == field/2 - Valid for tx
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0"); // N/2
    let v = 0x1c;
    let expected = hex_to_address("B29F65aA401660dfa96ecD7eB28134d87E9a618D");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #35 failed");

    // #37 s == field (N) - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #37 should fail (s == N)");

    // #39 s == field + 1 - should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364142");
    let v = 0x1c;

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert!(result.is_err(), "Test #39 should fail (s > N)");

    /////////
    // Additional edge case tests
    /////////
    // #40 ECGX
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eebe");
    let r = hex_to_32("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"); // ECGX
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c4");
    let v = 0x1b;
    let expected = hex_to_address("3c8b5f2426549658f425fad7e061c500626b39a3");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #40 failed");

    // #41 -ECGX
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eebe");
    let r = hex_to_32("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"); // ECGX
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c4");
    let v = 0x1c;
    let expected = hex_to_address("687526ad233c19b0a9b2eb8c96d1a45603f3919e");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Test #41 failed");

    // Additional edge case with 2G point
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eec0");
    let r = hex_to_32("c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5");
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c8");
    let v = 0x1b;
    let expected = hex_to_address("89fea198bd4a0efb63f36b5a04c993cca3a1891d");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Edge case 2G v=27 failed");

    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eec0");
    let r = hex_to_32("c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5");
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c8");
    let v = 0x1c;
    let expected = hex_to_address("c613182e8da092201765f852e3d6fbec4d281432");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Edge case 2G v=28 failed");

    // p: (0xc6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5,0x1ae168fea63dc339a3c58419466ceaeef7f632653266d0e1236431a950cfe52a)
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eec1");
    let r = hex_to_32("c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5");
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c8");
    let v = 0x1b;
    let expected = hex_to_address("1772bf1cfa310fdca361ee65825a74013eeaa17e");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Edge case 2G hash+1 failed");

    // Masked ECGX tests
    // P2_C0_EGX = ECGX & 0xFFFF
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eebe");
    let r = hex_to_32("0000000000000000000000000000000000000000000000000000000000001798"); // ECGX & 0xFFFF
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c4");
    let v = 0x1b;
    let expected = hex_to_address("9446d37b3aaedc97b2a8a9437998ba5787a2d5cb");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Masked ECGX 0xFFFF test failed");

    // P2_CH_EGX = ECGX & 0x001FFFF...FFFF (248 bits)
    let hash = hex_to_32("3cc4cb050478c49877188e4fbd022f35ccb41cee02d9d4417194cbf7ebc1eebe");
    let r = hex_to_32("001e667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"); // ECGX & 0x001FFF...
    let s = hex_to_32("2bcf13b5e4f34a04a77344d5943e0228ba2e787583036325450c889d597a16c4");
    let v = 0x1b;
    let expected = hex_to_address("4c90563674ab8de6f7731475a01a2bd09fd7b4b1");

    let sig = build_sig_65(r, s, v);
    let result = crypto.recover_signer_unchecked(&sig, &hash);
    assert_eq!(result.unwrap(), expected, "Masked ECGX 248-bit test failed");

    println!("All EcRecover Tx tests passed!");
}

pub fn ecrecover_precompile_tests(crypto: &CustomEvmCrypto) {
    // #34 s == field/2 + 1. Valid for precompile
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a1"); // N/2 + 1
    let v = 0x1c;
    let recid = v_to_recid(v);
    let expected = hex_to_address("4ef445CADd8bEe8A02bc79b30A97e6Fe3AE3B7a3");

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    let output = result.expect("Test #34 precompile should succeed");
    let recovered_address = Address::from_slice(&output[12..]);
    assert_eq!(recovered_address, expected, "Test #34 precompile failed");

    // #36 s == field/2 + 2. Valid for precompile
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a2"); // N/2 + 2
    let v = 0x1c;
    let recid = v_to_recid(v);
    let expected = hex_to_address("fE706AA7fe3455F29e0F5553D9C780Be3Bd54564");

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    let output = result.expect("Test #36 precompile should succeed");
    let recovered_address = Address::from_slice(&output[12..]);
    assert_eq!(recovered_address, expected, "Test #36 precompile failed");

    // #38 s == field - 1. Valid for precompile
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140"); // N - 1
    let v = 0x1c;
    let recid = v_to_recid(v);
    let expected = hex_to_address("c846e2E4Ab85A761042265B9A8d995345432A60e");

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    let output = result.expect("Test #38 precompile should succeed");
    let recovered_address = Address::from_slice(&output[12..]);
    assert_eq!(recovered_address, expected, "Test #38 precompile failed");

    // Point at infinity test - should fail
    let hash = hex_to_32("0000000000000000000000000000000000000000000000000000000000000001");
    let r = hex_to_32("c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5");
    let s = hex_to_32("7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a1");
    let v = 0x1b;
    let recid = v_to_recid(v);

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    assert!(result.is_err(), "Point at infinity test should fail");

    // Additional precompile tests with valid low S (should also work)
    let hash = hex_to_32("d9eba16ed0ecae432b71fe008c98cc872bb4cc214d3220a36f365326cf807d68");
    let r = hex_to_32("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"); // ECGX
    let s = hex_to_32("265e99e47ad31bb2cab9646c504576b3abc6939a1710afc08cbf3034d73214b8");
    let v = 0x1c;
    let recid = v_to_recid(v);
    let expected = hex_to_address("BC44674AD5868F642EAD3FDF94E2D9C9185EAFB7");

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    let output = result.expect("Precompile with low S should succeed");
    let recovered_address = Address::from_slice(&output[12..]);
    assert_eq!(recovered_address, expected, "Precompile with low S failed");

    // Precompile: r == 0 should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("0000000000000000000000000000000000000000000000000000000000000000");
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;
    let recid = v_to_recid(v);

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    assert!(result.is_err(), "Precompile r==0 should fail");

    // Precompile: s == 0 should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("0000000000000000000000000000000000000000000000000000000000000000");
    let v = 0x1c;
    let recid = v_to_recid(v);

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    assert!(result.is_err(), "Precompile s==0 should fail");

    // Precompile: r >= N should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"); // N
    let s = hex_to_32("4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
    let v = 0x1c;
    let recid = v_to_recid(v);

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    assert!(result.is_err(), "Precompile r>=N should fail");

    // Precompile: s >= N should fail
    let hash = hex_to_32("456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
    let r = hex_to_32("9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
    let s = hex_to_32("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"); // N
    let v = 0x1c;
    let recid = v_to_recid(v);

    let sig = build_sig(r, s);
    let result = crypto.secp256k1_ecrecover(&sig, recid, &hash);
    assert!(result.is_err(), "Precompile s>=N should fail");

    // Geth test vectors
    let tests = parse_precompile_json(include_str!("../testdata/ecRecover.json"));
    for test in &tests {
        let t = match parse_ecrecover_test(test) {
            Ok(t) => t,
            Err(e) => {
                assert!(
                    matches!(test.expected, ExpectedOutcome::Success(ref b) if b.is_empty())
                        || matches!(test.expected, ExpectedOutcome::Failure(_)),
                    "ecRecover {} has invalid input ({}) but expects success",
                    test.name,
                    e
                );
                continue;
            }
        };

        // The JSON uses the raw precompile interface (v as 27/28)
        let recid = t.v - 27;
        let sig = build_sig(t.r, t.s);

        match t.expected {
            Some(expected_padded) => {
                let result = crypto.secp256k1_ecrecover(&sig, recid, &t.hash);
                assert!(result.is_ok(), "ecRecover {} should succeed", t.name);
                let output = result.unwrap();
                assert_eq!(
                    output,
                    expected_padded,
                    "ecRecover {} mismatch:\n  got = 0x{}\n  expected = 0x{}",
                    t.name,
                    hex::encode(output),
                    hex::encode(expected_padded)
                );
            }
            None => {
                // Expected to fail OR return empty
                let result = crypto.secp256k1_ecrecover(&sig, recid, &t.hash);
                assert!(
                    result.is_err(),
                    "ecRecover {} should fail but got 0x{}",
                    t.name,
                    result.map(|r| hex::encode(r)).unwrap_or_default()
                );
            }
        }
    }

    println!("All EcRecover Precompile tests passed!");
}

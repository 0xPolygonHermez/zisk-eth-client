use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

/// Helper to convert hex string to fixed 32-byte array
fn hex_to_hash(hex: &str) -> [u8; 32] {
    let hex = hex.trim_start_matches("0x");
    let bytes = hex::decode(hex).expect("valid hex");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

pub fn sha256_tests(crypto: &CustomEvmCrypto) {
    sha256_basic_tests(crypto);
    sha256_unaligned_tests(crypto);
    sha256_nist_tests(crypto);
    sha256_length_tests(crypto);
    println!("All SHA256 tests passed!");
}

fn sha256_basic_tests(crypto: &CustomEvmCrypto) {
    // Single character
    // SHA256("a") = ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb
    let result = crypto.sha256(b"a");
    assert_eq!(
        result,
        hex_to_hash("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"),
        "sha256('a')"
    );

    // "abc" - NIST test vector
    // SHA256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let result = crypto.sha256(b"abc");
    assert_eq!(
        result,
        hex_to_hash("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        "sha256('abc')"
    );

    // "hello world"
    // SHA256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    let result = crypto.sha256(b"hello world");
    assert_eq!(
        result,
        hex_to_hash("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"),
        "sha256('hello world')"
    );

    // Longer string
    // SHA256("The quick brown fox jumps over the lazy dog")
    let result = crypto.sha256(b"The quick brown fox jumps over the lazy dog");
    assert_eq!(
        result,
        hex_to_hash("d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"),
        "sha256('The quick brown fox...')"
    );

    // With period at end (different hash!)
    // SHA256("The quick brown fox jumps over the lazy dog.")
    let result = crypto.sha256(b"The quick brown fox jumps over the lazy dog.");
    assert_eq!(
        result,
        hex_to_hash("ef537f25c895bfa782526529a9b63d97aa631564d5d789c2b765448c8635fb6c"),
        "sha256('The quick brown fox....')"
    );

    // With numbers
    let result = crypto.sha256(b"hello world 1234");
    assert_eq!(
        result,
        hex_to_hash("87472796b6bd3ab3651bd9a5f7306d84397eb2c6cf99e477fa9c37e9d7c6d6bb"),
        "sha256('hello world 1234')"
    );
}

fn sha256_nist_tests(crypto: &CustomEvmCrypto) {
    // Official NIST test vectors from FIPS 180-4

    // NIST Short Message Test: "abc"
    let result = crypto.sha256(b"abc");
    assert_eq!(
        result,
        hex_to_hash("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        "NIST: sha256('abc')"
    );

    // NIST Short Message Test: "" (empty)
    let result = crypto.sha256(b"");
    assert_eq!(
        result,
        hex_to_hash("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        "NIST: sha256('')"
    );

    // NIST Long Message Test: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq" (448 bits)
    let result = crypto.sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
    assert_eq!(
        result,
        hex_to_hash("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
        "NIST: sha256('abcdbcdecdefdefg...')"
    );

    // NIST Long Message Test: "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu" (896 bits)
    let result = crypto.sha256(b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu");
    assert_eq!(
        result,
        hex_to_hash("cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"),
        "NIST: sha256('abcdefghbcdefghi...')"
    );
}

fn sha256_length_tests(crypto: &CustomEvmCrypto) {
    // Test various input lengths around block boundaries
    // SHA256 uses 64-byte (512-bit) blocks

    // Exactly 1 byte
    let result = crypto.sha256(&[0x00]);
    assert_eq!(
        result,
        hex_to_hash("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"),
        "sha256(0x00)"
    );

    // 31 bytes (just under 32)
    let input: Vec<u8> = (0..31).collect();
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("4f23c2ca8c5c962e50cd31e221bfb6d0adca19111dca8e0c62598ff146dd19c4"),
        "sha256(0x00..0x1e)"
    );

    // 32 bytes (common hash size)
    let input: Vec<u8> = (0..32).collect();
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("630dcd2966c4336691125448bbb25b4ff412a49c732db2c8abc1b8581bd710dd"),
        "sha256(0x00..0x1f)"
    );

    // 55 bytes (max message length that fits in one block with padding)
    let input: Vec<u8> = vec![0xAB; 55];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("48d76eab30e51201f4f03ec7a85dab8510fb3409ccd15b54767f9b4435c9f54d"),
        "sha256(55 bytes of 0xAB)"
    );

    // 56 bytes (requires two blocks due to padding)
    let input: Vec<u8> = vec![0xAB; 56];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("a8c9906ade2a2eff868fd8f97a570bbc01a13cddc32c3dfdc9a18f0618d69e55"),
        "sha256(56 bytes of 0xAB)"
    );

    // 63 bytes (one less than block size)
    let input: Vec<u8> = vec![0xAB; 63];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("d1036ba30d050c74b1a5ab301fa29ff0c607a27cc55af3412577f7e06dbd190b"),
        "sha256(63 bytes of 0xAB)"
    );

    // 64 bytes (exactly one block)
    let input: Vec<u8> = vec![0xAB; 64];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("ec65c8798ecf95902413c40f7b9e6d4b0068885f5f324aba1f9ba1c8e14aea61"),
        "sha256(64 bytes of 0xAB)"
    );

    // 65 bytes (one more than block size)
    let input: Vec<u8> = vec![0xAB; 65];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("39cd843414d5125dd308568ace26d04e60b7fa6d2b1a901fb5184fa2eae0598b"),
        "sha256(65 bytes of 0xAB)"
    );

    // 128 bytes (exactly two blocks)
    let input: Vec<u8> = vec![0xCD; 128];
    let result = crypto.sha256(&input);
    assert_eq!(
        result,
        hex_to_hash("885ef0783ff857466de0f1464fa28cf1fd9a9abaed41a330068966c6e3525505"),
        "sha256(128 bytes of 0xCD)"
    );
}

fn sha256_unaligned_tests(crypto: &CustomEvmCrypto) {
    // 1] Use a fixed-size array with padding to create unaligned slice
    let aligned_buffer: [u8; 64] = [
        0x00, // padding byte at offset 0
        0x61, 0x62, 0x63, // "abc"
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let unaligned_slice = &aligned_buffer[1..4];
    let result = crypto.sha256(unaligned_slice);
    assert_eq!(
        result,
        hex_to_hash("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        "sha256('abc') from unaligned offset 1"
    );

    // 2] Use a Vec with explicit unaligned access via raw pointer
    let mut buffer = vec![0u8; 128];
    buffer[1] = b'h';
    buffer[2] = b'e';
    buffer[3] = b'l';
    buffer[4] = b'l';
    buffer[5] = b'o';

    let unaligned_hello = &buffer[1..6];
    let result = crypto.sha256(unaligned_hello);
    assert_eq!(
        result,
        hex_to_hash("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
        "sha256('hello') from unaligned offset"
    );

    // 3] Test various alignment offsets
    for offset in [1usize, 2, 3, 5, 7] {
        let mut buf = vec![0u8; 64 + offset];
        // Write "abc" at the offset
        buf[offset] = b'a';
        buf[offset + 1] = b'b';
        buf[offset + 2] = b'c';

        let slice = &buf[offset..offset + 3];
        let result = crypto.sha256(slice);
        assert_eq!(
            result,
            hex_to_hash("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            "sha256('abc') from offset {offset}"
        );
    }

    // 4] Larger unaligned data
    let mut large_buf = vec![0u8; 256];
    for i in 0..64 {
        large_buf[3 + i] = i as u8;
    }

    // Hash 64 bytes from unaligned offset 3
    let unaligned_64 = &large_buf[3..67];
    let result = crypto.sha256(unaligned_64);
    // This should match sha256(0x00, 0x01, 0x02, ..., 0x3f)
    assert_eq!(
        result,
        hex_to_hash("fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108"),
        "sha256(0..64 bytes) from unaligned offset 3"
    );
}

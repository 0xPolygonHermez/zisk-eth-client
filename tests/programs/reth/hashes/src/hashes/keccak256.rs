unsafe extern "C" {
    // This gets linked to the ziskos keccak256 implementation
    fn native_keccak256(bytes: *const u8, len: usize, output: *mut u8);
}
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    unsafe { native_keccak256(bytes.as_ptr(), bytes.len(), output.as_mut_ptr().cast::<u8>()) };
    output
}

/// Helper to convert hex string to fixed 32-byte array
fn hex_to_hash(hex: &str) -> [u8; 32] {
    let hex = hex.trim_start_matches("0x");
    let bytes = hex::decode(hex).expect("valid hex");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

pub fn keccak256_tests() {
    keccak256_basic_tests();
    keccak256_length_tests();
    println!("All Keccak256 tests passed!");
}

fn keccak256_basic_tests() {
    // Empty input - this is a critical edge case
    // keccak256("") = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
    let result = keccak256(b"");
    assert_eq!(
        result,
        hex_to_hash("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
        "keccak256 of empty string"
    );

    // Single character
    // keccak256("a") = 0x3ac225168df54212a25c1c01fd35bebfea408fdac2e31ddd6f80a4bbf9a5f1cb
    let result = keccak256(b"a");
    assert_eq!(
        result,
        hex_to_hash("3ac225168df54212a25c1c01fd35bebfea408fdac2e31ddd6f80a4bbf9a5f1cb"),
        "keccak256('a')"
    );

    // "abc" - common test vector
    // keccak256("abc") = 0x4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45
    let result = keccak256(b"abc");
    assert_eq!(
        result,
        hex_to_hash("4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"),
        "keccak256('abc')"
    );

    // "hello world"
    // keccak256("hello world") = 0x47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad
    let result = keccak256(b"hello world");
    assert_eq!(
        result,
        hex_to_hash("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad"),
        "keccak256('hello world')"
    );

    // Longer string
    // keccak256("The quick brown fox jumps over the lazy dog")
    let result = keccak256(b"The quick brown fox jumps over the lazy dog");
    assert_eq!(
        result,
        hex_to_hash("4d741b6f1eb29cb2a9b9911c82f56fa8d73b04959d3d9d222895df6c0b28aa15"),
        "keccak256('The quick brown fox...')"
    );

    // With numbers
    let result = keccak256(b"hello world 1234");
    assert_eq!(
        result,
        hex_to_hash("95788534752a0a1a1cc3cc7872031c3a3cc421296eb117fc9f657f8a2480efc1"),
        "keccak256('hello world 1234')"
    );
}

fn keccak256_length_tests() {
    // Test various input lengths around block boundaries
    // Keccak256 uses 136-byte blocks (1088 bits = rate for Keccak-256)

    // Exactly 1 byte
    let result = keccak256(&[0x00]);
    assert_eq!(
        result,
        hex_to_hash("bc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a"),
        "keccak256(0x00)"
    );

    // 31 bytes (just under 32)
    let input: Vec<u8> = (0..31).collect();
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("3e50547cf72e8583ee91462f9d99fe624f53282f78e1a5ec2347b1d0123d0d9b"),
        "keccak256(0x00..0x1e)"
    );

    // 32 bytes (common hash size)
    let input: Vec<u8> = (0..32).collect();
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("8ae1aa597fa146ebd3aa2ceddf360668dea5e526567e92b0321816a4e895bd2d"),
        "keccak256(0x00..0x1f)"
    );

    // 64 bytes (two hash outputs)
    let input: Vec<u8> = (0..64).collect();
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("002030bde3d4cf89919649775cd71875c4d0ab1708a380e03fefc3a28aa24831"),
        "keccak256(0x00..0x3f)"
    );

    // 135 bytes (one less than block size)
    let input: Vec<u8> = vec![0xAB; 135];
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("932fedc0e854cc4d32eec69e896c7449570052b3aaceacff7b13745325e4cf47"),
        "keccak256(135 bytes of 0xAB)"
    );

    // 136 bytes (exactly one block)
    let input: Vec<u8> = vec![0xAB; 136];
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("302db73a4c8cc8ecc9004fec3a6525d9d6a2dd4b098b1bf62d1b897acff18c9d"),
        "keccak256(136 bytes of 0xAB)"
    );

    // 137 bytes (one more than block size)
    let input: Vec<u8> = vec![0xAB; 137];
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("0235d14cb2563be9d300a26aa4dd02e37e51b802b1b204691db2de6a329d7948"),
        "keccak256(137 bytes of 0xAB)"
    );

    // 272 bytes (exactly two blocks)
    let input: Vec<u8> = vec![0xCD; 272];
    let result = keccak256(&input);
    assert_eq!(
        result,
        hex_to_hash("82b861ac327cd75d750316c8d35ec36dec8d6eb049289fa8cc3638f90d428efc"),
        "keccak256(272 bytes of 0xCD)"
    );
}

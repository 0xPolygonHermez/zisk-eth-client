/// Helper to convert a decimal string to 32-byte big-endian array
pub fn decimal_to_32(dec: &str) -> [u8; 32] {
    let n = dec.parse::<num_bigint::BigUint>().expect("valid decimal");
    let bytes = n.to_bytes_be();
    let mut arr = [0u8; 32];
    let start = 32 - bytes.len();
    arr[start..].copy_from_slice(&bytes);
    arr
}

/// Helper to build a G1 point (64 bytes) from x and y coordinates (decimal strings)
pub fn build_g1_point(x: &str, y: &str) -> [u8; 64] {
    let mut point = [0u8; 64];
    point[..32].copy_from_slice(&decimal_to_32(x));
    point[32..].copy_from_slice(&decimal_to_32(y));
    point
}

// Helper to build a G2 point (128 bytes) from x1, x2, y1, y2 coordinates (decimal strings)
// G2 point format: x1 (32 bytes) || x2 (32 bytes) || y1 (32 bytes) || y2 (32 bytes)
pub fn build_g2_point(x1: &str, x2: &str, y1: &str, y2: &str) -> [u8; 128] {
    let mut point = [0u8; 128];
    point[..32].copy_from_slice(&decimal_to_32(x1));
    point[32..64].copy_from_slice(&decimal_to_32(x2));
    point[64..96].copy_from_slice(&decimal_to_32(y1));
    point[96..].copy_from_slice(&decimal_to_32(y2));
    point
}

/// Helper to check if result is the point at infinity (all zeros)
pub fn is_infinity(result: &[u8; 64]) -> bool {
    result.iter().all(|&b| b == 0)
}

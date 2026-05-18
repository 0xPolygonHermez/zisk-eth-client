pub type G1Point = ([u8; 48], [u8; 48]);
pub type G2Point = ([u8; 48], [u8; 48], [u8; 48], [u8; 48]);
pub type G1PointScalar = (G1Point, [u8; 32]);
pub type G2PointScalar = (G2Point, [u8; 32]);

/// Parse a 64-byte padded field element to 48 bytes (strips 16-byte zero prefix)
pub fn parse_fp_padded(input: &[u8]) -> Result<[u8; 48], String> {
    if input.len() != 64 {
        return Err(format!("Invalid padded field element length: {}", input.len()));
    }

    let (padding, data) = input.split_at(16);
    if padding.iter().any(|&b| b != 0) {
        return Err("Invalid padding: non-zero bytes in padding".to_string());
    }

    Ok(data.try_into().unwrap())
}

/// Parse a 128-byte padded G1 point to unpadded G1Point (two 48-byte coordinates)
pub fn parse_g1_point_padded(input: &[u8]) -> Result<G1Point, String> {
    if input.len() != 128 {
        return Err(format!("Invalid G1 point length: {}", input.len()));
    }

    let x = parse_fp_padded(&input[0..64])?;
    let y = parse_fp_padded(&input[64..128])?;
    Ok((x, y))
}

/// Parse a 256-byte padded G2 point to unpadded G2Point (four 48-byte coordinates)
pub fn parse_g2_point_padded(input: &[u8]) -> Result<G2Point, String> {
    if input.len() != 256 {
        return Err(format!("Invalid G2 point length: {}", input.len()));
    }

    let x0 = parse_fp_padded(&input[0..64])?;
    let x1 = parse_fp_padded(&input[64..128])?;
    let y0 = parse_fp_padded(&input[128..192])?;
    let y1 = parse_fp_padded(&input[192..256])?;
    Ok((x0, x1, y0, y1))
}

/// Convert 96-byte unpadded result to 128-byte padded format
pub fn pad_g1_result(unpadded: &[u8; 96]) -> [u8; 128] {
    let mut padded = [0u8; 128];
    padded[16..64].copy_from_slice(&unpadded[0..48]);
    padded[80..128].copy_from_slice(&unpadded[48..96]);
    padded
}

/// Convert 192-byte unpadded result to 256-byte padded format
pub fn pad_g2_result(unpadded: &[u8; 192]) -> [u8; 256] {
    let mut padded = [0u8; 256];
    padded[16..64].copy_from_slice(&unpadded[0..48]);
    padded[80..128].copy_from_slice(&unpadded[48..96]);
    padded[144..192].copy_from_slice(&unpadded[96..144]);
    padded[208..256].copy_from_slice(&unpadded[144..192]);
    padded
}

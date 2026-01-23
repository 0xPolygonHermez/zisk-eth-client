use revm::precompile::{
    bls12_381::{G1Point, G1PointScalar, G2Point, G2PointScalar},
    Crypto, PrecompileError,
};

use alloy_consensus::crypto::{CryptoProvider, RecoveryError};
use alloy_primitives::Address;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn sha256_c(input: *const u8, input_len: usize, output: *mut u8);

    fn bn254_g1_add_c(p1: *const u8, p2: *const u8, ret: *mut u8) -> u8;

    fn bn254_g1_mul_c(point: *const u8, scalar: *const u8, ret: *mut u8) -> u8;

    fn bn254_pairing_check_c(pairs: *const u8, num_pairs: usize) -> u8;

    fn secp256k1_ecrecover_c(
        sig: *const u8,
        recid: u8,
        msg: *const u8,
        output: *mut u8,
        require_low_s: bool,
    ) -> u8;

    fn modexp_bytes_c(
        base_ptr: *const u8,
        base_len: usize,
        exp_ptr: *const u8,
        exp_len: usize,
        modulus_ptr: *const u8,
        modulus_len: usize,
        ret_ptr: *mut u8,
    ) -> usize;

    fn verify_kzg_proof_c(
        z: *const u8,
        y: *const u8,
        commitment: *const u8,
        proof: *const u8,
    ) -> bool;

    fn bls12_381_g1_add_c(ret: *mut u8, a: *const u8, b: *const u8) -> u8;

    fn bls12_381_g1_msm_c(ret: *mut u8, pairs: *const u8, num_pairs: usize) -> u8;

    fn bls12_381_g2_add_c(ret: *mut u8, a: *const u8, b: *const u8) -> u8;

    fn bls12_381_g2_msm_c(ret: *mut u8, pairs: *const u8, num_pairs: usize) -> u8;

    fn bls12_381_pairing_check_c(pairs: *const u8, num_pairs: usize) -> u8;
}

#[derive(Debug, Default)]
pub struct CustomEvmCrypto;

impl Crypto for CustomEvmCrypto {
    /// Compute SHA-256 hash
    #[inline]
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut output = [0u8; 32];
            unsafe {
                sha256_c(input.as_ptr(), input.len(), output.as_mut_ptr());
            }
            output
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = input;
            unimplemented!();
        }
    }

    // /// Compute RIPEMD-160 hash
    // #[inline]
    // fn ripemd160(&self, input: &[u8]) -> [u8; 32] {
    //     use ripemd::Digest;
    //     let mut hasher = ripemd::Ripemd160::new();
    //     hasher.update(input);

    //     let mut output = [0u8; 32];
    //     hasher.finalize_into((&mut output[12..]).into());
    //     output
    // }

    /// BN254 elliptic curve addition.
    #[inline]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u8; 64];
            let ret = unsafe { bn254_g1_add_c(p1.as_ptr(), p2.as_ptr(), result.as_mut_ptr()) };
            match ret {
                0 | 1 => Ok(result),
                2 => Err(PrecompileError::other("bn254_g1_add inputs not in field")),
                3 => Err(PrecompileError::Bn254FieldPointNotAMember),
                _ => Err(PrecompileError::other("bn254_g1_add failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (p1, p2);
            unimplemented!();
        }
    }

    /// BN254 elliptic curve scalar multiplication.
    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u8; 64];
            let ret =
                unsafe { bn254_g1_mul_c(point.as_ptr(), scalar.as_ptr(), result.as_mut_ptr()) };
            match ret {
                0 | 1 => Ok(result), // 0=success, 1=success_infinity
                2 => Err(PrecompileError::other("bn254_g1_mul inputs not in field")),
                3 => Err(PrecompileError::Bn254FieldPointNotAMember),
                _ => Err(PrecompileError::other("bn254_g1_mul failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (point, scalar);
            unimplemented!();
        }
    }

    /// BN254 pairing check.
    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // Each pair is G1 (64 bytes) + G2 (128 bytes) = 192 bytes
            let mut pairs_bytes = Vec::new();

            for (g1, g2) in pairs {
                pairs_bytes.extend_from_slice(g1);
                pairs_bytes.extend_from_slice(g2);
            }

            let ret = unsafe { bn254_pairing_check_c(pairs_bytes.as_ptr(), pairs.len()) };
            match ret {
                0 => Ok(true),
                1 => Ok(false),
                2 => Err(PrecompileError::other("bn254 G1 inputs not in field")),
                3 => Err(PrecompileError::Bn254FieldPointNotAMember),
                4 => Err(PrecompileError::other("bn254 G2 inputs not in field")),
                5 => Err(PrecompileError::other("bn254 G2 point not on curve")),
                6 => Err(PrecompileError::other(
                    "bn254 pairing check subgroup check failed",
                )),
                _ => Err(PrecompileError::other("bn254_pairing_check failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = pairs;
            unimplemented!();
        }
    }

    /// secp256k1 ECDSA signature recovery.
    #[inline]
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut output = [0u8; 32];
            let ret = unsafe {
                secp256k1_ecrecover_c(
                    sig.as_ptr(),
                    recid,
                    msg.as_ptr(),
                    output.as_mut_ptr(),
                    false,
                )
            };
            match ret {
                0 => Ok(output),
                _ => Err(PrecompileError::Secp256k1RecoverFailed),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (sig, recid, msg);
            unimplemented!();
        }
    }

    /// Modular exponentiation.
    #[inline]
    fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = vec![0u8; modulus.len()];
            unsafe {
                modexp_bytes_c(
                    base.as_ptr(),
                    base.len(),
                    exp.as_ptr(),
                    exp.len(),
                    modulus.as_ptr(),
                    modulus.len(),
                    result.as_mut_ptr(),
                );
            }
            Ok(result)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (base, exp, modulus);
            unimplemented!();
        }
    }

    // /// Blake2 compression function.
    // #[inline]
    // fn blake2_compress(&self, rounds: u32, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
    //     crate::blake2::algo::compress(rounds as usize, h, m, t, f);
    // }

    // /// secp256r1 (P-256) signature verification.
    // #[inline]
    // fn secp256r1_verify_signature(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
    //     crate::secp256r1::verify_signature(*msg, *sig, *pk).is_some()
    // }

    /// KZG point evaluation.
    #[inline]
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let valid = unsafe {
                verify_kzg_proof_c(z.as_ptr(), y.as_ptr(), commitment.as_ptr(), proof.as_ptr())
            };
            if !valid {
                return Err(PrecompileError::BlobVerifyKzgProofFailed);
            }
            Ok(())
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (z, y, commitment, proof);
            unimplemented!();
        }
    }

    /// BLS12-381 G1 addition (returns 96-byte unpadded G1 point)
    fn bls12_381_g1_add(&self, a: G1Point, b: G1Point) -> Result<[u8; 96], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // G1Point is ([u8; 48], [u8; 48])
            let mut a_bytes = [0u8; 96];
            a_bytes[..48].copy_from_slice(&a.0);
            a_bytes[48..].copy_from_slice(&a.1);

            let mut b_bytes = [0u8; 96];
            b_bytes[..48].copy_from_slice(&b.0);
            b_bytes[48..].copy_from_slice(&b.1);

            let mut result = [0u8; 96];
            let ret_code = unsafe {
                bls12_381_g1_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
            };

            match ret_code {
                0 | 1 => Ok(result),
                _ => Err(PrecompileError::Bls12381G1NotOnCurve),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (a, b);
            unimplemented!();
        }
    }

    /// BLS12-381 G1 multi-scalar multiplication (returns 96-byte unpadded G1 point)
    fn bls12_381_g1_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G1PointScalar, PrecompileError>>,
    ) -> Result<[u8; 96], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // G1PointScalar is (G1Point, [u8; 32]) = (([u8; 48], [u8; 48]), [u8; 32])
            // Each pair is 96 + 32 = 128 bytes
            let mut pairs_bytes = Vec::new();
            let mut num_pairs = 0usize;
            for pair in pairs {
                let (point, scalar) = pair?;
                pairs_bytes.extend_from_slice(&point.0);
                pairs_bytes.extend_from_slice(&point.1);
                pairs_bytes.extend_from_slice(&scalar);
                num_pairs += 1;
            }

            let mut result = [0u8; 96];
            let ret_code =
                unsafe { bls12_381_g1_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs) };

            match ret_code {
                0 | 1 => Ok(result),
                2 => Err(PrecompileError::Bls12381G1NotOnCurve),
                3 => Err(PrecompileError::Bls12381G1NotInSubgroup),
                _ => Err(PrecompileError::other("bls12_381_g1_msm failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = pairs;
            unimplemented!();
        }
    }

    /// BLS12-381 G2 addition (returns 192-byte unpadded G2 point)
    fn bls12_381_g2_add(&self, a: G2Point, b: G2Point) -> Result<[u8; 192], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // G2Point is ([u8; 48], [u8; 48], [u8; 48], [u8; 48])
            let mut a_bytes = [0u8; 192];
            a_bytes[..48].copy_from_slice(&a.0);
            a_bytes[48..96].copy_from_slice(&a.1);
            a_bytes[96..144].copy_from_slice(&a.2);
            a_bytes[144..].copy_from_slice(&a.3);

            let mut b_bytes = [0u8; 192];
            b_bytes[..48].copy_from_slice(&b.0);
            b_bytes[48..96].copy_from_slice(&b.1);
            b_bytes[96..144].copy_from_slice(&b.2);
            b_bytes[144..].copy_from_slice(&b.3);

            let mut result = [0u8; 192];
            let ret_code = unsafe {
                bls12_381_g2_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
            };
            match ret_code {
                0 | 1 => Ok(result),
                _ => Err(PrecompileError::Bls12381G2NotOnCurve),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (a, b);
            unimplemented!();
        }
    }

    /// BLS12-381 G2 multi-scalar multiplication (returns 192-byte unpadded G2 point)
    fn bls12_381_g2_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G2PointScalar, PrecompileError>>,
    ) -> Result<[u8; 192], PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // G2PointScalar is (G2Point, [u8; 32]) = (([u8; 48], [u8; 48], [u8; 48], [u8; 48]), [u8; 32])
            // Each pair is 192 + 32 = 224 bytes
            let mut pairs_bytes = Vec::new();
            let mut num_pairs = 0usize;

            for pair in pairs {
                let (point, scalar) = pair?;
                pairs_bytes.extend_from_slice(&point.0);
                pairs_bytes.extend_from_slice(&point.1);
                pairs_bytes.extend_from_slice(&point.2);
                pairs_bytes.extend_from_slice(&point.3);
                pairs_bytes.extend_from_slice(&scalar);
                num_pairs += 1;
            }

            let mut result = [0u8; 192];
            let ret_code =
                unsafe { bls12_381_g2_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs) };
            match ret_code {
                0 | 1 => Ok(result),
                2 => Err(PrecompileError::Bls12381G2NotOnCurve),
                3 => Err(PrecompileError::Bls12381G2NotInSubgroup),
                _ => Err(PrecompileError::other("bls12_381_g2_msm failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = pairs;
            unimplemented!();
        }
    }

    /// BLS12-381 pairing check.
    fn bls12_381_pairing_check(
        &self,
        pairs: &[(G1Point, G2Point)],
    ) -> Result<bool, PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // Each pair is G1Point (96 bytes) + G2Point (192 bytes) = 288 bytes
            let mut pairs_bytes = Vec::new();
            for (g1, g2) in pairs {
                // G1Point: ([u8; 48], [u8; 48])
                pairs_bytes.extend_from_slice(&g1.0);
                pairs_bytes.extend_from_slice(&g1.1);
                // G2Point: ([u8; 48], [u8; 48], [u8; 48], [u8; 48])
                pairs_bytes.extend_from_slice(&g2.0);
                pairs_bytes.extend_from_slice(&g2.1);
                pairs_bytes.extend_from_slice(&g2.2);
                pairs_bytes.extend_from_slice(&g2.3);
            }

            let ret_code = unsafe { bls12_381_pairing_check_c(pairs_bytes.as_ptr(), pairs.len()) };
            match ret_code {
                0 => Ok(true),
                1 => Ok(false),
                2 => Err(PrecompileError::Bls12381G1NotOnCurve),
                3 => Err(PrecompileError::Bls12381G1NotInSubgroup),
                4 => Err(PrecompileError::Bls12381G2NotOnCurve),
                5 => Err(PrecompileError::Bls12381G2NotInSubgroup),
                _ => Err(PrecompileError::other("bls12_381_pairing_check failed")),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = pairs;
            unimplemented!();
        }
    }

    // /// BLS12-381 map field element to G1.
    // fn bls12_381_fp_to_g1(&self, fp: &[u8; 48]) -> Result<[u8; 96], PrecompileError> {
    //     crate::bls12_381::crypto_backend::map_fp_to_g1_bytes(fp)
    // }

    // /// BLS12-381 map field element to G2.
    // fn bls12_381_fp2_to_g2(&self, fp2: ([u8; 48], [u8; 48])) -> Result<[u8; 192], PrecompileError> {
    //     crate::bls12_381::crypto_backend::map_fp2_to_g2_bytes(&fp2.0, &fp2.1)
    // }
}

impl CryptoProvider for CustomEvmCrypto {
    /// Recover signer from signature and message hash, without ensuring low S values.
    fn recover_signer_unchecked(
        &self,
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // Extract signature (first 64 bytes) and recovery id (last byte)
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&sig[..64]);
            let recid = sig[64];

            let mut output = [0u8; 32];
            let ret = unsafe {
                secp256k1_ecrecover_c(
                    sig_bytes.as_ptr(),
                    recid,
                    msg.as_ptr(),
                    output.as_mut_ptr(),
                    true,
                )
            };
            if ret != 0 {
                return Err(RecoveryError::new());
            }
            // The output is already the keccak256 hash of the public key (last 20 bytes = address)
            Ok(Address::from_slice(&output[12..]))
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            let _ = (sig, msg);
            unimplemented!();
        }
    }
}

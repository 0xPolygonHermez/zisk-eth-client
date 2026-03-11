use ethereum_types::Address;

use ethrex_crypto::{Crypto, CryptoError};

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use guest_common::ffi::*;
#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints))]
use guest_common::ffi::*;
#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints_debug))]
use guest_common::ffi::*;

use super::ZiskCrypto;

impl Crypto for ZiskCrypto {
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], CryptoError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut output = [0u8; 32];
            let ret = unsafe {
                secp256k1_ecdsa_address_recover_c(
                    sig.as_ptr(),
                    recid,
                    msg.as_ptr(),
                    output.as_mut_ptr(),
                )
            };
            match ret {
                0 => Ok(output),
                _ => Err(CryptoError::RecoveryFailed),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.secp256k1_ecrecover(sig, recid, msg)
        }
    }

    fn recover_signer(&self, sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, CryptoError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // Extract signature (first 64 bytes) and recovery id (last byte)
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&sig[..64]);
            let recid = sig[64];

            let mut output = [0u8; 32];
            let ret = unsafe {
                secp256k1_ecdsa_address_recover_c(
                    sig_bytes.as_ptr(),
                    recid,
                    msg.as_ptr(),
                    output.as_mut_ptr(),
                )
            };
            match ret {
                0 => {
                    // The output is already the keccak256 hash of the public key (last 20 bytes = address)
                    Ok(Address::from_slice(&output[12..]))
                }
                _ => Err(CryptoError::RecoveryFailed),
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.recover_signer(sig, msg)
        }
    }

    // fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], CryptoError> {
    //     substrate_bn_g1_add(p1, p2)
    // }

    // fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], CryptoError> {
    //     substrate_bn_g1_mul(point, scalar)
    // }

    // fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, CryptoError> {
    //     substrate_bn_pairing_check(pairs)
    // }

    // /// ZisK-accelerated modular exponentiation via native circuit instruction.
    // fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, CryptoError> {
    //     let modulus_len = modulus.len();

    //     if modulus.iter().all(|&b| b == 0) {
    //         return Ok(vec![0u8; modulus_len]);
    //     }

    //     if exp.iter().all(|&b| b == 0) {
    //         // base^0 mod m = 1 mod m
    //         let mod_is_one =
    //             modulus.iter().rev().skip(1).all(|&b| b == 0) && modulus.last() == Some(&1);
    //         let mut result = vec![0u8; modulus_len];
    //         if !mod_is_one && modulus_len > 0 {
    //             #[allow(clippy::indexing_slicing)]
    //             {
    //                 result[modulus_len - 1] = 1;
    //             }
    //         }
    //         return Ok(result);
    //     }

    //     let base_limbs = bytes_be_to_limbs_asc(base);
    //     let exp_limbs = bytes_be_to_limbs_asc(exp);
    //     let modulus_limbs = bytes_be_to_limbs_asc(modulus);

    //     let result_limbs = ziskos::zisklib::modexp_u64(&base_limbs, &exp_limbs, &modulus_limbs);

    //     let result_bytes = limbs_asc_to_bytes_be(&result_limbs);

    //     let mut out = vec![0u8; modulus_len];
    //     #[allow(clippy::indexing_slicing)]
    //     if result_bytes.len() <= modulus_len {
    //         let offset = modulus_len - result_bytes.len();
    //         out[offset..].copy_from_slice(&result_bytes);
    //     } else {
    //         out.copy_from_slice(&result_bytes[result_bytes.len() - modulus_len..]);
    //     }
    //     Ok(out)
    // }

    // /// ZisK-accelerated 256-bit modular multiplication via native circuit instruction.
    // fn mulmod256(&self, a: &[u8; 32], b: &[u8; 32], m: &[u8; 32]) -> [u8; 32] {
    //     use ethereum_types::U256;

    //     let m_u256 = U256::from_big_endian(m);
    //     if m_u256.is_zero() {
    //         return [0u8; 32];
    //     }

    //     let a_u256 = U256::from_big_endian(a);
    //     let b_u256 = U256::from_big_endian(b);

    //     let mut result = U256::zero();
    //     // SAFETY: ziskos FFI is safe when called with valid 4-element u64 arrays.
    //     // U256::0 is [u64; 4] in little-endian word order, matching ziskos ABI.
    //     unsafe {
    //         ziskos::zisklib::mulmod256_c(
    //             a_u256.0.as_ptr(),
    //             b_u256.0.as_ptr(),
    //             m_u256.0.as_ptr(),
    //             result.0.as_mut_ptr(),
    //         );
    //     }
    //     result.to_big_endian()
    // }
}

// impl Crypto for ZiskCrypto {
//     /// Compute SHA-256 hash
//     #[inline]
//     fn sha256(&self, input: &[u8]) -> [u8; 32] {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_sha256(input.as_ptr(), input.len());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!("hint_sha2 (input: {:x?})", &input));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut output = [0u8; 32];
//                 unsafe {
//                     sha256_c(input.as_ptr(), input.len(), output.as_mut_ptr());
//                 }
//                 output
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.sha256(input)
//         }
//     }

//     // /// Compute RIPEMD-160 hash
//     // #[inline]
//     // fn ripemd160(&self, input: &[u8]) -> [u8; 32] {
//     //     use ripemd::Digest;
//     //     let mut hasher = ripemd::Ripemd160::new();
//     //     hasher.update(input);

//     //     let mut output = [0u8; 32];
//     //     hasher.finalize_into((&mut output[12..]).into());
//     //     output
//     // }

//     /// BN254 elliptic curve addition.
//     #[inline]
//     fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bn254_g1_add(p1.as_ptr(), p2.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bn254_g1_add (p1: {:x?}, p2: {:x?})",
//                 &p1, &p2
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 64];
//                 let ret = unsafe { bn254_g1_add_c(p1.as_ptr(), p2.as_ptr(), result.as_mut_ptr()) };
//                 match ret {
//                     0 | 1 => Ok(result),
//                     2 => Err(PrecompileError::other("bn254_g1_add inputs not in field")),
//                     3 => Err(PrecompileError::Bn254FieldPointNotAMember),
//                     _ => Err(PrecompileError::other("bn254_g1_add failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bn254_g1_add(p1, p2)
//         }
//     }

//     /// BN254 elliptic curve scalar multiplication.
//     #[inline]
//     fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bn254_g1_mul(point.as_ptr(), scalar.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bn254_g1_mul (point: {:x?}, scalar: {:x?})",
//                 &point, &scalar
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 64];
//                 let ret =
//                     unsafe { bn254_g1_mul_c(point.as_ptr(), scalar.as_ptr(), result.as_mut_ptr()) };
//                 match ret {
//                     0 | 1 => Ok(result), // 0=success, 1=success_infinity
//                     2 => Err(PrecompileError::other("bn254_g1_mul inputs not in field")),
//                     3 => Err(PrecompileError::Bn254FieldPointNotAMember),
//                     _ => Err(PrecompileError::other("bn254_g1_mul failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bn254_g1_mul(point, scalar)
//         }
//     }

//     /// BN254 pairing check.
//     #[inline]
//     fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // Each pair is G1 (64 bytes) + G2 (128 bytes) = 192 bytes
//             let mut pairs_bytes = Vec::new();

//             for (g1, g2) in pairs {
//                 pairs_bytes.extend_from_slice(g1);
//                 pairs_bytes.extend_from_slice(g2);
//             }

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bn254_pairing_check(pairs_bytes.as_ptr(), pairs.len());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bn254_pairing_check (pairs_bytes: {:x?}), num_pairs: {}",
//                 &pairs_bytes[..],
//                 pairs.len()
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let ret = unsafe { bn254_pairing_check_c(pairs_bytes.as_ptr(), pairs.len()) };
//                 match ret {
//                     0 => Ok(true),
//                     1 => Ok(false),
//                     2 => Err(PrecompileError::other("bn254 G1 inputs not in field")),
//                     3 => Err(PrecompileError::Bn254FieldPointNotAMember),
//                     4 => Err(PrecompileError::other("bn254 G2 inputs not in field")),
//                     5 => Err(PrecompileError::other("bn254 G2 point not on curve")),
//                     6 => Err(PrecompileError::other(
//                         "bn254 pairing check subgroup check failed",
//                     )),
//                     _ => Err(PrecompileError::other("bn254_pairing_check failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bn254_pairing_check(pairs)
//         }
//     }

//     /// secp256k1 ECDSA signature recovery.
//     #[inline]
//     fn secp256k1_ecrecover(
//         &self,
//         sig: &[u8; 64],
//         recid: u8,
//         msg: &[u8; 32],
//     ) -> Result<[u8; 32], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 let recid_bytes = (recid as u64).to_le_bytes();
//                 hint_secp256k1_ecdsa_address_recover(
//                     sig.as_ptr(),
//                     recid_bytes.as_ptr(),
//                     msg.as_ptr(),
//                 );
//             }

//             #[cfg(zisk_hints_debug)]
//             {
//                 let recid_bytes = (recid as u64).to_le_bytes();
//                 hint_log(format!(
//                     "hint_secp256k1_ecdsa_address_recover (sig: {:x?}, recid: {:x?}, msg: {:x?})",
//                     &sig, &recid_bytes, &msg
//                 ));
//             }

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut output = [0u8; 32];
//                 let ret = unsafe {
//                     secp256k1_ecdsa_address_recover_c(
//                         sig.as_ptr(),
//                         recid,
//                         msg.as_ptr(),
//                         output.as_mut_ptr(),
//                     )
//                 };
//                 match ret {
//                     0 => Ok(output),
//                     _ => Err(PrecompileError::Secp256k1RecoverFailed),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             // Pause hint emission here so default_crypto.secp256k1_ecrecover cannot produce extra hints (e.g. keccak256)
//             #[cfg(zisk_hints)]
//             let already_paused = unsafe { pause_hints() };

//             let result = self.default_crypto.secp256k1_ecrecover(sig, recid, msg);

//             #[cfg(zisk_hints)]
//             {
//                 if !already_paused {
//                     unsafe { resume_hints() };
//                 }
//             }

//             result
//         }
//     }

//     /// Modular exponentiation.
//     #[inline]
//     fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_modexp_bytes(
//                     base.as_ptr(),
//                     base.len(),
//                     exp.as_ptr(),
//                     exp.len(),
//                     modulus.as_ptr(),
//                     modulus.len(),
//                 );
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_modexp_bytes (base: {:x?}, exp: {:x?}, modulus: {:x?})",
//                 &base, &exp, &modulus
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = vec![0u8; modulus.len()];
//                 unsafe {
//                     modexp_bytes_c(
//                         base.as_ptr(),
//                         base.len(),
//                         exp.as_ptr(),
//                         exp.len(),
//                         modulus.as_ptr(),
//                         modulus.len(),
//                         result.as_mut_ptr(),
//                     );
//                 }
//                 Ok(result)
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.modexp(base, exp, modulus)
//         }
//     }

//     /// Blake2 compression function.
//     #[inline]
//     fn blake2_compress(&self, rounds: u32, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_blake2b_compress(rounds, h.as_mut_ptr(), m.as_ptr(), t.as_ptr(), f as u8);
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_blake2b_compress (rounds: {:x?}, h: {:x?}, m: {:x?}, t: {:x?}, f: {:x?})",
//                 &rounds, &h, &m, &t, &f
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             unsafe {
//                 blake2b_compress_c(rounds, h.as_mut_ptr(), m.as_ptr(), t.as_ptr(), f as u8);
//             }
//         }

//         #[cfg(not(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints)))]
//         {
//             self.default_crypto.blake2_compress(rounds, h, m, t, f);
//         }
//     }

//     /// secp256r1 (P-256) signature verification.
//     #[inline]
//     fn secp256r1_verify_signature(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_secp256r1_ecdsa_verify(msg.as_ptr(), sig.as_ptr(), pk.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_secp256r1_ecdsa_verify (msg: {:x?}, sig: {:x?}, pk: {:x?})",
//                 &msg, &sig, &pk
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 unsafe { secp256r1_ecdsa_verify_c(msg.as_ptr(), sig.as_ptr(), pk.as_ptr()) }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.secp256r1_verify_signature(msg, sig, pk)
//         }
//     }

//     /// KZG point evaluation.
//     #[inline]
//     fn verify_kzg_proof(
//         &self,
//         z: &[u8; 32],
//         y: &[u8; 32],
//         commitment: &[u8; 48],
//         proof: &[u8; 48],
//     ) -> Result<(), PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_verify_kzg_proof(z.as_ptr(), y.as_ptr(), commitment.as_ptr(), proof.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_verify_kzg_proof (z: {:x?}, y: {:x?}, commitment: {:x?}, proof: {:x?})",
//                 &z, &y, &commitment, &proof
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let valid = unsafe {
//                     verify_kzg_proof_c(z.as_ptr(), y.as_ptr(), commitment.as_ptr(), proof.as_ptr())
//                 };
//                 if !valid {
//                     return Err(PrecompileError::BlobVerifyKzgProofFailed);
//                 }
//                 Ok(())
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto
//                 .verify_kzg_proof(z, y, commitment, proof)
//         }
//     }

//     /// BLS12-381 G1 addition (returns 96-byte unpadded G1 point)
//     fn bls12_381_g1_add(&self, a: G1Point, b: G1Point) -> Result<[u8; 96], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // G1Point is ([u8; 48], [u8; 48])
//             let mut a_bytes = [0u8; 96];
//             a_bytes[..48].copy_from_slice(&a.0);
//             a_bytes[48..].copy_from_slice(&a.1);

//             let mut b_bytes = [0u8; 96];
//             b_bytes[..48].copy_from_slice(&b.0);
//             b_bytes[48..].copy_from_slice(&b.1);

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_g1_add(a_bytes.as_ptr(), b_bytes.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bls12_381_g1_add (a: {:x?}, b: {:x?})",
//                 &a_bytes, &b_bytes
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 96];
//                 let ret_code = unsafe {
//                     bls12_381_g1_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
//                 };

//                 match ret_code {
//                     0 | 1 => Ok(result),
//                     _ => Err(PrecompileError::Bls12381G1NotOnCurve),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bls12_381_g1_add(a, b)
//         }
//     }

//     /// BLS12-381 G1 multi-scalar multiplication (returns 96-byte unpadded G1 point)
//     fn bls12_381_g1_msm(
//         &self,
//         pairs: &mut dyn Iterator<Item = Result<G1PointScalar, PrecompileError>>,
//     ) -> Result<[u8; 96], PrecompileError> {
//         // TODO: Review if it's a better way to do this to avoid borrowing issues with pairs
//         let mut collected: Vec<G1PointScalar> = Vec::new();
//         for pair in pairs {
//             collected.push(pair?);
//         }

//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // G1PointScalar is (G1Point, [u8; 32]) = (([u8; 48], [u8; 48]), [u8; 32])
//             // Each pair is 96 + 32 = 128 bytes

//             let mut pairs_bytes = Vec::new();
//             let mut num_pairs = 0usize;

//             for (point, scalar) in &collected {
//                 pairs_bytes.extend_from_slice(&point.0);
//                 pairs_bytes.extend_from_slice(&point.1);
//                 pairs_bytes.extend_from_slice(scalar);
//                 num_pairs += 1;
//             }

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_g1_msm(pairs_bytes.as_ptr(), num_pairs);
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bls12_381_g1_msm (pairs_bytes: {:x?}, num_pairs: {})",
//                 &pairs_bytes[..],
//                 num_pairs
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 96];
//                 let ret_code = unsafe {
//                     bls12_381_g1_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs)
//                 };

//                 match ret_code {
//                     0 | 1 => Ok(result),
//                     2 => Err(PrecompileError::Bls12381G1NotOnCurve),
//                     3 => Err(PrecompileError::Bls12381G1NotInSubgroup),
//                     _ => Err(PrecompileError::other("bls12_381_g1_msm failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             let mut it = collected.into_iter().map(Ok);
//             self.default_crypto.bls12_381_g1_msm(&mut it)
//         }
//     }

//     /// BLS12-381 G2 addition (returns 192-byte unpadded G2 point)
//     fn bls12_381_g2_add(&self, a: G2Point, b: G2Point) -> Result<[u8; 192], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // G2Point is ([u8; 48], [u8; 48], [u8; 48], [u8; 48])
//             let mut a_bytes = [0u8; 192];
//             a_bytes[..48].copy_from_slice(&a.0);
//             a_bytes[48..96].copy_from_slice(&a.1);
//             a_bytes[96..144].copy_from_slice(&a.2);
//             a_bytes[144..].copy_from_slice(&a.3);

//             let mut b_bytes = [0u8; 192];
//             b_bytes[..48].copy_from_slice(&b.0);
//             b_bytes[48..96].copy_from_slice(&b.1);
//             b_bytes[96..144].copy_from_slice(&b.2);
//             b_bytes[144..].copy_from_slice(&b.3);

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_g2_add(a_bytes.as_ptr(), b_bytes.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bls12_381_g2_add (a: {:x?}, b: {:x?})",
//                 &a_bytes, &b_bytes
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 192];
//                 let ret_code = unsafe {
//                     bls12_381_g2_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
//                 };
//                 match ret_code {
//                     0 | 1 => Ok(result),
//                     _ => Err(PrecompileError::Bls12381G2NotOnCurve),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bls12_381_g2_add(a, b)
//         }
//     }

//     /// BLS12-381 G2 multi-scalar multiplication (returns 192-byte unpadded G2 point)
//     fn bls12_381_g2_msm(
//         &self,
//         pairs: &mut dyn Iterator<Item = Result<G2PointScalar, PrecompileError>>,
//     ) -> Result<[u8; 192], PrecompileError> {
//         // TODO: Review if it's a better way to do this to avoid borrowing issues with pairs
//         let mut collected: Vec<G2PointScalar> = Vec::new();
//         for pair in pairs {
//             collected.push(pair?);
//         }

//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // G2PointScalar is (G2Point, [u8; 32]) = (([u8; 48], [u8; 48], [u8; 48], [u8; 48]), [u8; 32])
//             // Each pair is 192 + 32 = 224 bytes
//             let mut pairs_bytes = Vec::new();
//             let mut num_pairs = 0usize;

//             for (point, scalar) in &collected {
//                 pairs_bytes.extend_from_slice(&point.0);
//                 pairs_bytes.extend_from_slice(&point.1);
//                 pairs_bytes.extend_from_slice(&point.2);
//                 pairs_bytes.extend_from_slice(&point.3);
//                 pairs_bytes.extend_from_slice(scalar);
//                 num_pairs += 1;
//             }

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_g2_msm(pairs_bytes.as_ptr(), num_pairs);
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bls12_381_g2_msm (pairs_bytes: {:x?}, num_pairs: {})",
//                 &pairs_bytes[..],
//                 num_pairs
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 192];
//                 let ret_code = unsafe {
//                     bls12_381_g2_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs)
//                 };
//                 match ret_code {
//                     0 | 1 => Ok(result),
//                     2 => Err(PrecompileError::Bls12381G2NotOnCurve),
//                     3 => Err(PrecompileError::Bls12381G2NotInSubgroup),
//                     _ => Err(PrecompileError::other("bls12_381_g2_msm failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             let mut it = collected.into_iter().map(Ok);
//             self.default_crypto.bls12_381_g2_msm(&mut it)
//         }
//     }

//     /// BLS12-381 pairing check.
//     fn bls12_381_pairing_check(
//         &self,
//         pairs: &[(G1Point, G2Point)],
//     ) -> Result<bool, PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             // Each pair is G1Point (96 bytes) + G2Point (192 bytes) = 288 bytes
//             let mut pairs_bytes = Vec::new();
//             for (g1, g2) in pairs {
//                 // G1Point: ([u8; 48], [u8; 48])
//                 pairs_bytes.extend_from_slice(&g1.0);
//                 pairs_bytes.extend_from_slice(&g1.1);
//                 // G2Point: ([u8; 48], [u8; 48], [u8; 48], [u8; 48])
//                 pairs_bytes.extend_from_slice(&g2.0);
//                 pairs_bytes.extend_from_slice(&g2.1);
//                 pairs_bytes.extend_from_slice(&g2.2);
//                 pairs_bytes.extend_from_slice(&g2.3);
//             }

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_pairing_check(pairs_bytes.as_ptr(), pairs.len());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!(
//                 "hint_bls12_381_pairing_check (pairs_bytes: {:x?}, num_pairs: {})",
//                 &pairs_bytes,
//                 pairs.len()
//             ));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let ret_code =
//                     unsafe { bls12_381_pairing_check_c(pairs_bytes.as_ptr(), pairs.len()) };
//                 match ret_code {
//                     0 => Ok(true),
//                     1 => Ok(false),
//                     2 => Err(PrecompileError::Bls12381G1NotOnCurve),
//                     3 => Err(PrecompileError::Bls12381G1NotInSubgroup),
//                     4 => Err(PrecompileError::Bls12381G2NotOnCurve),
//                     5 => Err(PrecompileError::Bls12381G2NotInSubgroup),
//                     _ => Err(PrecompileError::other("bls12_381_pairing_check failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bls12_381_pairing_check(pairs)
//         }
//     }

//     /// BLS12-381 map field element to G1.
//     fn bls12_381_fp_to_g1(&self, fp: &[u8; 48]) -> Result<[u8; 96], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_fp_to_g1(fp.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!("hint_bls12_381_fp_to_g1 (fp: {:x?})", &fp));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 96];
//                 let ret_code = unsafe { bls12_381_fp_to_g1_c(result.as_mut_ptr(), fp.as_ptr()) };
//                 match ret_code {
//                     0 => Ok(result),
//                     _ => Err(PrecompileError::other("bls12_381_fp_to_g1 failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bls12_381_fp_to_g1(fp)
//         }
//     }

//     /// BLS12-381 map field element to G2.
//     fn bls12_381_fp2_to_g2(&self, fp2: ([u8; 48], [u8; 48])) -> Result<[u8; 192], PrecompileError> {
//         #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
//         {
//             let mut fp2_bytes = [0u8; 96];
//             fp2_bytes[..48].copy_from_slice(&fp2.0);
//             fp2_bytes[48..].copy_from_slice(&fp2.1);

//             #[cfg(zisk_hints)]
//             unsafe {
//                 hint_bls12_381_fp2_to_g2(fp2_bytes.as_ptr());
//             }

//             #[cfg(zisk_hints_debug)]
//             hint_log(format!("hint_bls12_381_fp2_to_g2 (fp2: {:x?})", &fp2_bytes));

//             #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
//             {
//                 let mut result = [0u8; 192];
//                 let ret_code =
//                     unsafe { bls12_381_fp2_to_g2_c(result.as_mut_ptr(), fp2_bytes.as_ptr()) };
//                 match ret_code {
//                     0 => Ok(result),
//                     _ => Err(PrecompileError::other("bls12_381_fp2_to_g2 failed")),
//                 }
//             }
//         }

//         #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
//         {
//             self.default_crypto.bls12_381_fp2_to_g2(fp2)
//         }
//     }
// }

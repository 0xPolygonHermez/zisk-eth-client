use revm::precompile::{
    bls12_381::{G1Point, G1PointScalar, G2Point, G2PointScalar},
    Crypto, PrecompileError, DefaultCrypto,
};

use alloy_consensus::crypto::{CryptoProvider, RecoveryError};
use alloy_primitives::Address;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn sha256_c(input: *const u8, input_len: usize, output: *mut u8);

    fn modexp_bytes_c(
        base_ptr: *const u8,
        base_len: usize,
        exp_ptr: *const u8,
        exp_len: usize,
        modulus_ptr: *const u8,
        modulus_len: usize,
        result_ptr: *mut u8,
    ) -> usize;

    fn bls12_381_g1_add_c(ret: *mut u8, a: *const u8, b: *const u8) -> u8;

    fn bls12_381_g1_msm_c(ret: *mut u8, pairs: *const u8, num_pairs: usize) -> u8;

    fn bls12_381_g2_add_c(ret: *mut u8, a: *const u8, b: *const u8) -> u8;

    fn bls12_381_g2_msm_c(ret: *mut u8, pairs: *const u8, num_pairs: usize) -> u8;

    fn bls12_381_pairing_check_c(pairs: *const u8, num_pairs: usize) -> bool;

    fn verify_kzg_proof_c(
        z: *const u8,
        y: *const u8,
        commitment: *const u8,
        proof: *const u8,
    ) -> bool;

    fn bn254_g1_add_c(p1: *const u8, p2: *const u8, result: *mut u8) -> u8;

    fn bn254_g1_mul_c(point: *const u8, scalar: *const u8, result: *mut u8) -> u8;

    fn bn254_pairing_check_c(
        g1_ptrs: *const *const u8,
        g2_ptrs: *const *const u8,
        num_pairs: usize,
    ) -> bool;

    fn secp256k1_ecrecover_c(sig: *const u8, recid: u8, msg: *const u8, output: *mut u8) -> u8;
}

#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints))]
extern "C" {
    fn hint_sha2(f: *const u8);
    fn hint_bn254_g1_add(p1: *const u8, p2: *const u8);
    fn hint_bn254_g1_mul(point: *const u8, scalar: *const u8);
    fn hint_bls12_381_g1_add(a: *const u8, b: *const u8);
    fn hint_bls12_381_g2_add(a: *const u8, b: *const u8);
    fn hint_secp256k1_ecrecover(sig: *const u8, recid: u8, msg: *const u8);
}

#[derive(Debug)]
pub struct CustomEvmCrypto {
    pub default_crypto: DefaultCrypto,
}

impl Default for CustomEvmCrypto {
    fn default() -> Self {
        Self { default_crypto: DefaultCrypto }
    }
}

impl Crypto for CustomEvmCrypto {
    /// Compute SHA-256 hash
    #[inline]
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_sha2(input.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("hint_sha2 (input: {:x?})", &input);

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
            self.default_crypto.sha256(input)
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
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_bn254_g1_add(p1.as_ptr(), p2.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("bn254_g1_add (p1: {:x?}, p2: {:x?})", &p1, &p2);

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = [0u8; 64];
                let ret = unsafe { bn254_g1_add_c(p1.as_ptr(), p2.as_ptr(), result.as_mut_ptr()) };
                if ret != 0 {
                    return Err(PrecompileError::other("bn254_g1_add failed"));
                }
                Ok(result)
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bn254_g1_add(p1, p2)
        }
    }

    /// BN254 elliptic curve scalar multiplication.
    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_bn254_g1_mul(point.as_ptr(), scalar.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("bn254_g1_mul (point: {:x?}, scalar: {:x?})", &point, &scalar);

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = [0u8; 64];
                let ret =
                    unsafe { bn254_g1_mul_c(point.as_ptr(), scalar.as_ptr(), result.as_mut_ptr()) };
                if ret != 0 {
                    return Err(PrecompileError::other("bn254_g1_mul failed"));
                }
                Ok(result)
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bn254_g1_mul(point, scalar)
        }
    }

    /// BN254 pairing check.
    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let g1_ptrs: Vec<*const u8> = pairs.iter().map(|(g1, _)| g1.as_ptr()).collect();
            let g2_ptrs: Vec<*const u8> = pairs.iter().map(|(_, g2)| g2.as_ptr()).collect();

            let ret =
                unsafe { bn254_pairing_check_c(g1_ptrs.as_ptr(), g2_ptrs.as_ptr(), pairs.len()) };
            Ok(ret)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bn254_pairing_check(pairs)
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
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_secp256k1_ecrecover(sig.as_ptr(), recid, msg.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("secp256k1_ecrecover (sig: {:x?}, recid: {}, msg: {:x?})", &sig, recid, &msg);

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut output = [0u8; 32];
                let ret = unsafe {
                    secp256k1_ecrecover_c(sig.as_ptr(), recid, msg.as_ptr(), output.as_mut_ptr())
                };
                if ret != 0 {
                    return Err(PrecompileError::Secp256k1RecoverFailed);
                }
                Ok(output)
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.secp256k1_ecrecover(sig, recid, msg)
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
            self.default_crypto.modexp(base, exp, modulus)
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
            self.default_crypto.verify_kzg_proof(z, y, commitment, proof)
        }
    }

    /// BLS12-381 G1 addition (returns 96-byte unpadded G1 point)
    fn bls12_381_g1_add(&self, a: G1Point, b: G1Point) -> Result<[u8; 96], PrecompileError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // G1Point is ([u8; 48], [u8; 48])
            let mut a_bytes = [0u8; 96];
            a_bytes[..48].copy_from_slice(&a.0);
            a_bytes[48..].copy_from_slice(&a.1);

            let mut b_bytes = [0u8; 96];
            b_bytes[..48].copy_from_slice(&b.0);
            b_bytes[48..].copy_from_slice(&b.1);

            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_bls12_381_g1_add(a_bytes.as_ptr(), b_bytes.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("hint_bls12_381_g1_add (a: {:x?}, b: {:x?})", &a_bytes, &b_bytes);

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = [0u8; 96];
                let ret = unsafe {
                    bls12_381_g1_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
                };
                if ret != 0 {
                    return Err(PrecompileError::other("bls12_381_g1_add failed"));
                }
                Ok(result)
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bls12_381_g1_add(a, b)
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
            let ret =
                unsafe { bls12_381_g1_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs) };
            if ret != 0 {
                return Err(PrecompileError::other("bls12_381_g1_msm failed"));
            }
            Ok(result)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bls12_381_g1_msm(pairs)
        }
    }

    /// BLS12-381 G2 addition (returns 192-byte unpadded G2 point)
    fn bls12_381_g2_add(&self, a: G2Point, b: G2Point) -> Result<[u8; 192], PrecompileError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
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

            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            unsafe { hint_bls12_381_g2_add(a_bytes.as_ptr(), b_bytes.as_ptr()); }

            #[cfg(zisk_hints_debug)]
            println!("hint_bls12_381_g2_add (a: {:x?}, b: {:x?})", &a_bytes, &b_bytes);

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = [0u8; 192];
                let ret = unsafe {
                    bls12_381_g2_add_c(result.as_mut_ptr(), a_bytes.as_ptr(), b_bytes.as_ptr())
                };
                if ret != 0 {
                    return Err(PrecompileError::other("bls12_381_g2_add failed"));
                }
                Ok(result)
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bls12_381_g2_add(a, b)
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
            let ret =
                unsafe { bls12_381_g2_msm_c(result.as_mut_ptr(), pairs_bytes.as_ptr(), num_pairs) };
            if ret != 0 {
                return Err(PrecompileError::other("bls12_381_g2_msm failed"));
            }
            Ok(result)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bls12_381_g2_msm(pairs)
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

            let ret = unsafe { bls12_381_pairing_check_c(pairs_bytes.as_ptr(), pairs.len()) };
            Ok(ret)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.default_crypto.bls12_381_pairing_check(pairs)
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
        // TODO: I can use RecoveryError to manage the errors
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            // Extract signature (first 64 bytes) and recovery id (last byte)
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&sig[..64]);
            let recid = sig[64];

            let mut output = [0u8; 32];
            let ret = unsafe {
                secp256k1_ecrecover_c(sig_bytes.as_ptr(), recid, msg.as_ptr(), output.as_mut_ptr())
            };
            if ret != 0 {
                return Err(RecoveryError::new());
            }
            // The output is already the keccak256 hash of the public key (last 20 bytes = address)
            Ok(Address::from_slice(&output[12..]))
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            use alloy_consensus::crypto::secp256k1::public_key_to_address;
            use k256::ecdsa::{RecoveryId, VerifyingKey};

            let mut signature = match k256::ecdsa::Signature::from_slice(&sig[0..64]){
                Ok(sig) => sig,
                Err(_) => return Err(RecoveryError::new()),
            };
            let mut recid = sig[64];

            // normalize signature and flip recovery id if needed.
            if let Some(sig_normalized) = signature.normalize_s() {
                signature = sig_normalized;
                recid ^= 1;
            }
            let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");

            // recover key
            let recovered_key = match VerifyingKey::recover_from_prehash(&msg[..], &signature, recid) {
                Ok(key) => key,
                Err(_) => return Err(RecoveryError::new()),
            };

            Ok(public_key_to_address(recovered_key))
        }
    }
}

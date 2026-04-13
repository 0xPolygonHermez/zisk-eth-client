use ethereum_types::Address;
use ethrex_common::Uint256Ops;
use ethrex_crypto::{Crypto, CryptoError};
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use tiny_keccak::{Hasher, Keccak};

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use guest_common::ffi::*;
#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints))]
use guest_common::ffi::*;
#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints_debug))]
use guest_common::ffi::*;

use super::ZiskAccelerator;

// Additional ZisK accelerators not declared in zkvm_accelerators.h.
// Declare them here so the linker can find them on the zkvm target.
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn mulmod256_c(a: *const u8, b: *const u8, m: *const u8, result: *mut u8);

    fn overflowing_add256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_sub256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_mul256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_pow256_c(base: *const u64, exp: *const u64, result: *mut u64) -> u8;

    fn checked_add256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_sub256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_mul256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_div256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_rem256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn saturating_add256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn saturating_sub256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn saturating_mul256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_div256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_rem256_c(a: *const u64, b: *const u64, result: *mut u64);
}

impl Crypto for ZiskAccelerator {
    fn ripemd160(&self, input: &[u8]) -> [u8; 32] {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_ripemd160(input.as_ptr(), input.len());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut output = zkvm_ripemd160_hash { data: [0u8; 32] };
                unsafe {
                    zkvm_ripemd160(input.as_ptr(), input.len(), &mut output);
                }
                return output.data;
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.ripemd160(input)
        }
    }

    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_sha256(input.as_ptr(), input.len());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut output = zkvm_sha256_hash { data: [0u8; 32] };
                unsafe {
                    zkvm_sha256(input.as_ptr(), input.len(), &mut output);
                }
                return output.data;
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.sha256(input)
        }
    }

    fn keccak256(&self, input: &[u8]) -> [u8; 32] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut output = zkvm_keccak256_hash { data: [0u8; 32] };
            unsafe {
                zkvm_keccak256(input.as_ptr(), input.len(), &mut output);
            }
            return output.data;
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.keccak256(input)
        }
    }

    fn blake2_compress(&self, rounds: u32, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_blake2b_compress(rounds, h.as_mut_ptr(), m.as_ptr(), t.as_ptr(), f as u8);
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            unsafe {
                zkvm_blake2f(
                    rounds,
                    h.as_mut_ptr() as *mut zkvm_blake2f_state,
                    m.as_ptr() as *const zkvm_blake2f_message,
                    t.as_ptr() as *const zkvm_blake2f_offset,
                    f as u8,
                );
                return;
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.blake2_compress(rounds, h, m, t, f);
        }
    }

    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                let recid_bytes = (recid as u64).to_le_bytes();
                hint_secp256k1_ecrecover(sig.as_ptr(), recid_bytes.as_ptr(), msg.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                // zkvm_secp256k1_ecrecover returns the raw 64-byte pubkey (x || y).
                // Compute keccak256(pubkey) in software to derive the address hash.
                let mut pubkey_out = zkvm_secp256k1_pubkey { data: [0u8; 64] };
                let ret = unsafe {
                    zkvm_secp256k1_ecrecover(
                        msg.as_ptr() as *const zkvm_secp256k1_hash,
                        sig.as_ptr() as *const zkvm_secp256k1_signature,
                        recid,
                        &mut pubkey_out,
                    )
                };
                return match ret {
                    0 => {
                        let mut hasher = Keccak::v256();
                        hasher.update(&pubkey_out.data);
                        let mut hash = [0u8; 32];
                        hasher.finalize(&mut hash);
                        // EVM spec: ecrecover output is a left-zero-padded address.
                        // First 12 bytes must be zero; only bytes 12..32 contain the address.
                        hash[..12].fill(0);
                        Ok(hash)
                    }
                    _ => Err(CryptoError::RecoveryFailed),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            // Pause hint emission so native ecrecover cannot produce extra hints (e.g. keccak256)
            #[cfg(zisk_hints)]
            let already_paused = unsafe { pause_hints() };

            let result = self.native_crypto.secp256k1_ecrecover(sig, recid, msg);

            #[cfg(zisk_hints)]
            {
                if !already_paused {
                    unsafe { resume_hints() };
                }
            }

            result
        }
    }

    fn recover_signer(&self, sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // Extract signature (first 64 bytes) and recovery id (last byte)
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&sig[..64]);
            let recid = sig[64];

            #[cfg(zisk_hints)]
            unsafe {
                let recid_bytes = (recid as u64).to_le_bytes();
                hint_secp256k1_ecrecover(sig_bytes.as_ptr(), recid_bytes.as_ptr(), msg.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut pubkey_out = zkvm_secp256k1_pubkey { data: [0u8; 64] };
                let ret = unsafe {
                    zkvm_secp256k1_ecrecover(
                        msg.as_ptr() as *const zkvm_secp256k1_hash,
                        sig_bytes.as_ptr() as *const zkvm_secp256k1_signature,
                        recid,
                        &mut pubkey_out,
                    )
                };
                return match ret {
                    0 => {
                        let mut hasher = Keccak::v256();
                        hasher.update(&pubkey_out.data);
                        let mut hash = [0u8; 32];
                        hasher.finalize(&mut hash);
                        Ok(Address::from_slice(&hash[12..]))
                    }
                    _ => Err(CryptoError::RecoveryFailed),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            #[cfg(zisk_hints)]
            let already_paused = unsafe { pause_hints() };

            let result = self.native_crypto.recover_signer(sig, msg);

            #[cfg(zisk_hints)]
            {
                if !already_paused {
                    unsafe { resume_hints() };
                }
            }

            result
        }
    }

    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_bn254_g1_add(p1.as_ptr(), p2.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bn254_g1_point { data: [0u8; 64] };
                let ret = unsafe {
                    zkvm_bn254_g1_add(
                        p1.as_ptr() as *const zkvm_bn254_g1_point,
                        p2.as_ptr() as *const zkvm_bn254_g1_point,
                        &mut result,
                    )
                };
                return match ret {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bn254_g1_add failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bn254_g1_add(p1, p2)
        }
    }

    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_bn254_g1_mul(point.as_ptr(), scalar.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bn254_g1_point { data: [0u8; 64] };
                let ret = unsafe {
                    zkvm_bn254_g1_mul(
                        point.as_ptr() as *const zkvm_bn254_g1_point,
                        scalar.as_ptr() as *const zkvm_bn254_scalar,
                        &mut result,
                    )
                };
                return match ret {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bn254_g1_mul failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bn254_g1_mul(point, scalar)
        }
    }

    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // Each pair is G1 (64 bytes) + G2 (128 bytes) = 192 bytes laid out contiguously.
            let mut pairs_bytes: Vec<u8> = Vec::with_capacity(pairs.len() * 192);
            for (g1, g2) in pairs {
                pairs_bytes.extend_from_slice(g1);
                pairs_bytes.extend_from_slice(g2);
            }

            #[cfg(zisk_hints)]
            unsafe {
                hint_bn254_pairing_check(pairs_bytes.as_ptr(), pairs.len());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut verified = false;
                let ret = unsafe {
                    zkvm_bn254_pairing(
                        pairs_bytes.as_ptr() as *const zkvm_bn254_pairing_pair,
                        pairs.len(),
                        &mut verified,
                    )
                };
                return match ret {
                    0 => Ok(verified),
                    _ => Err(CryptoError::Other("bn254_pairing_check failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bn254_pairing_check(pairs)
        }
    }

    fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_modexp_bytes(
                    base.as_ptr(),
                    base.len(),
                    exp.as_ptr(),
                    exp.len(),
                    modulus.as_ptr(),
                    modulus.len(),
                );
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = vec![0u8; modulus.len()];
                unsafe {
                    zkvm_modexp(
                        base.as_ptr(),
                        base.len(),
                        exp.as_ptr(),
                        exp.len(),
                        modulus.as_ptr(),
                        modulus.len(),
                        result.as_mut_ptr(),
                    );
                }
                return Ok(result);
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.modexp(base, exp, modulus)
        }
    }

    /// ZisK-accelerated 256-bit modular multiplication via native circuit instruction.
    fn mulmod256(&self, a: &[u8; 32], b: &[u8; 32], m: &[u8; 32]) -> [u8; 32] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u8; 32];
            unsafe {
                mulmod256_c(a.as_ptr(), b.as_ptr(), m.as_ptr(), result.as_mut_ptr());
            }
            return result;
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.mulmod256(a, b, m)
        }
    }

    fn secp256r1_verify(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_secp256r1_ecdsa_verify(msg.as_ptr(), sig.as_ptr(), pk.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut verified = false;
                unsafe {
                    zkvm_secp256r1_verify(
                        msg.as_ptr() as *const zkvm_secp256r1_hash,
                        sig.as_ptr() as *const zkvm_secp256r1_signature,
                        pk.as_ptr() as *const zkvm_secp256r1_pubkey,
                        &mut verified,
                    );
                }
                return verified;
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.secp256r1_verify(msg, sig, pk)
        }
    }

    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_verify_kzg_proof(z.as_ptr(), y.as_ptr(), commitment.as_ptr(), proof.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut verified = false;
                unsafe {
                    zkvm_kzg_point_eval(
                        commitment.as_ptr() as *const zkvm_kzg_commitment,
                        z.as_ptr() as *const zkvm_kzg_field_element,
                        y.as_ptr() as *const zkvm_kzg_field_element,
                        proof.as_ptr() as *const zkvm_kzg_proof,
                        &mut verified,
                    );
                }
                return if !verified {
                    Err(CryptoError::Other(
                        "KZG proof verification failed".to_string(),
                    ))
                } else {
                    Ok(())
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.verify_kzg_proof(z, y, commitment, proof)
        }
    }

    fn bls12_381_g1_add(
        &self,
        a: ([u8; 48], [u8; 48]),
        b: ([u8; 48], [u8; 48]),
    ) -> Result<[u8; 96], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // G1Point is ([u8; 48], [u8; 48])
            let mut a_bytes = [0u8; 96];
            a_bytes[..48].copy_from_slice(&a.0);
            a_bytes[48..].copy_from_slice(&a.1);

            let mut b_bytes = [0u8; 96];
            b_bytes[..48].copy_from_slice(&b.0);
            b_bytes[48..].copy_from_slice(&b.1);

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_g1_add(a_bytes.as_ptr(), b_bytes.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
                let ret_code = unsafe {
                    zkvm_bls12_g1_add(
                        a_bytes.as_ptr() as *const zkvm_bls12_381_g1_point,
                        b_bytes.as_ptr() as *const zkvm_bls12_381_g1_point,
                        &mut result,
                    )
                };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other(
                        "BLS12-381 G1 addition failed".to_string(),
                    )),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_g1_add(a, b)
        }
    }

    fn bls12_381_g1_msm(
        &self,
        pairs: &[(([u8; 48], [u8; 48]), [u8; 32])],
    ) -> Result<[u8; 96], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // Each pair is G1Point (96 bytes) || scalar (32 bytes) = 128 bytes.
            let num_pairs = pairs.len();
            let mut pairs_bytes: Vec<u8> = Vec::with_capacity(num_pairs * 128);
            for (point, scalar) in pairs {
                pairs_bytes.extend_from_slice(&point.0);
                pairs_bytes.extend_from_slice(&point.1);
                pairs_bytes.extend_from_slice(scalar);
            }

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_g1_msm(pairs_bytes.as_ptr(), num_pairs);
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
                let ret_code = unsafe {
                    zkvm_bls12_g1_msm(
                        pairs_bytes.as_ptr() as *const zkvm_bls12_381_g1_msm_pair,
                        num_pairs,
                        &mut result,
                    )
                };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bls12_381_g1_msm failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_g1_msm(pairs)
        }
    }

    fn bls12_381_g2_add(
        &self,
        a: ([u8; 48], [u8; 48], [u8; 48], [u8; 48]),
        b: ([u8; 48], [u8; 48], [u8; 48], [u8; 48]),
    ) -> Result<[u8; 192], CryptoError> {
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

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_g2_add(a_bytes.as_ptr(), b_bytes.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
                let ret_code = unsafe {
                    zkvm_bls12_g2_add(
                        a_bytes.as_ptr() as *const zkvm_bls12_381_g2_point,
                        b_bytes.as_ptr() as *const zkvm_bls12_381_g2_point,
                        &mut result,
                    )
                };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other(
                        "BLS12-381 G2 addition failed".to_string(),
                    )),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_g2_add(a, b)
        }
    }

    fn bls12_381_g2_msm(
        &self,
        pairs: &[(([u8; 48], [u8; 48], [u8; 48], [u8; 48]), [u8; 32])],
    ) -> Result<[u8; 192], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // Each pair is G2Point (192 bytes) || scalar (32 bytes) = 224 bytes.
            let num_pairs = pairs.len();
            let mut pairs_bytes: Vec<u8> = Vec::with_capacity(num_pairs * 224);
            for (point, scalar) in pairs {
                pairs_bytes.extend_from_slice(&point.0);
                pairs_bytes.extend_from_slice(&point.1);
                pairs_bytes.extend_from_slice(&point.2);
                pairs_bytes.extend_from_slice(&point.3);
                pairs_bytes.extend_from_slice(scalar);
            }

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_g2_msm(pairs_bytes.as_ptr(), num_pairs);
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
                let ret_code = unsafe {
                    zkvm_bls12_g2_msm(
                        pairs_bytes.as_ptr() as *const zkvm_bls12_381_g2_msm_pair,
                        num_pairs,
                        &mut result,
                    )
                };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bls12_381_g2_msm failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_g2_msm(pairs)
        }
    }

    fn bls12_381_pairing_check(
        &self,
        pairs: &[(
            ([u8; 48], [u8; 48]),
            ([u8; 48], [u8; 48], [u8; 48], [u8; 48]),
        )],
    ) -> Result<bool, CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            // Each pair is G1Point (96 bytes) || G2Point (192 bytes) = 288 bytes.
            let mut pairs_bytes: Vec<u8> = Vec::with_capacity(pairs.len() * 288);
            for (g1, g2) in pairs {
                pairs_bytes.extend_from_slice(&g1.0);
                pairs_bytes.extend_from_slice(&g1.1);
                pairs_bytes.extend_from_slice(&g2.0);
                pairs_bytes.extend_from_slice(&g2.1);
                pairs_bytes.extend_from_slice(&g2.2);
                pairs_bytes.extend_from_slice(&g2.3);
            }

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_pairing_check(pairs_bytes.as_ptr(), pairs.len());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut verified = false;
                let ret_code = unsafe {
                    zkvm_bls12_pairing(
                        pairs_bytes.as_ptr() as *const zkvm_bls12_381_pairing_pair,
                        pairs.len(),
                        &mut verified,
                    )
                };
                return match ret_code {
                    0 => Ok(verified),
                    _ => Err(CryptoError::Other(
                        "bls12_381_pairing_check failed".to_string(),
                    )),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_pairing_check(pairs)
        }
    }

    fn bls12_381_fp_to_g1(&self, fp: &[u8; 48]) -> Result<[u8; 96], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_fp_to_g1(fp.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
                let ret_code = unsafe {
                    zkvm_bls12_map_fp_to_g1(fp.as_ptr() as *const zkvm_bls12_381_fp, &mut result)
                };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bls12_381_fp_to_g1 failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_fp_to_g1(fp)
        }
    }

    fn bls12_381_fp2_to_g2(&self, fp2: ([u8; 48], [u8; 48])) -> Result<[u8; 192], CryptoError> {
        #[cfg(any(all(target_os = "zkvm", target_vendor = "zisk"), zisk_hints))]
        {
            let mut fp2_bytes = [0u8; 96];
            fp2_bytes[..48].copy_from_slice(&fp2.0);
            fp2_bytes[48..].copy_from_slice(&fp2.1);

            #[cfg(zisk_hints)]
            unsafe {
                hint_bls12_381_fp2_to_g2(fp2_bytes.as_ptr());
            }

            #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
            {
                let fp2_struct = zkvm_bls12_381_fp2 { data: fp2_bytes };
                let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
                let ret_code = unsafe { zkvm_bls12_map_fp2_to_g2(&fp2_struct, &mut result) };
                return match ret_code {
                    0 => Ok(result.data),
                    _ => Err(CryptoError::Other("bls12_381_fp2_to_g2 failed".to_string())),
                };
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_crypto.bls12_381_fp2_to_g2(fp2)
        }
    }
}

impl Uint256Ops for ZiskAccelerator {
    // TODO: Investigate
    // fn overflowing_add(&self, a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         let overflow =
    //             unsafe { overflowing_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         (result, overflow != 0)
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.overflowing_add(a, b)
    //     }
    // }

    // TODO: Investigate
    // fn overflowing_sub(&self, a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         let overflow =
    //             unsafe { overflowing_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         (result, overflow != 0)
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.overflowing_sub(a, b)
    //     }
    // }

    fn overflowing_mul(&self, a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.overflowing_mul(a, b)
        }
    }

    fn overflowing_pow(&self, base: [u64; 4], exp: [u64; 4]) -> ([u64; 4], bool) {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_pow256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.overflowing_pow(base, exp)
        }
    }

    // TODO: Investigate
    // fn checked_add(&self, a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         let success = unsafe { checked_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         if success == 1 {
    //             Some(result)
    //         } else {
    //             None
    //         }
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.checked_add(a, b)
    //     }
    // }

    // TODO: Investigate
    // fn checked_sub(&self, a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         let success = unsafe { checked_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         if success == 1 {
    //             Some(result)
    //         } else {
    //             None
    //         }
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.checked_sub(a, b)
    //     }
    // }

    fn checked_mul(&self, a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.checked_mul(a, b)
        }
    }

    fn checked_div(&self, a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_div256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.checked_div(a, b)
        }
    }

    fn checked_rem(&self, a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_rem256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.checked_rem(a, b)
        }
    }

    // TODO: Investigate
    // fn saturating_add(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         unsafe { saturating_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         result
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.saturating_add(a, b)
    //     }
    // }

    // TODO: Investigate
    // fn saturating_sub(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    //     #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    //     {
    //         let mut result = [0u64; 4];
    //         unsafe { saturating_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
    //         result
    //     }

    //     #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    //     {
    //         self.native_uint256_ops.saturating_sub(a, b)
    //     }
    // }

    fn saturating_mul(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.saturating_mul(a, b)
        }
    }

    // ── U256 bitwise & inspection ───────────────────────────────────

    // fn not(&self, a: [u64; 4]) -> [u64; 4] {
    //     [!a[0], !a[1], !a[2], !a[3]]
    // }

    // fn bitand(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    //     [a[0] & b[0], a[1] & b[1], a[2] & b[2], a[3] & b[3]]
    // }

    // fn bitor(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    //     [a[0] | b[0], a[1] | b[1], a[2] | b[2], a[3] | b[3]]
    // }

    // fn bitxor(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    //     [a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]]
    // }

    // fn shl(&self, a: [u64; 4], shift: usize) -> [u64; 4] {
    //     let a = ethereum_types::U256(a);
    //     (a << shift).0
    // }

    // fn shr(&self, a: [u64; 4], shift: usize) -> [u64; 4] {
    //     let a = ethereum_types::U256(a);
    //     (a >> shift).0
    // }

    fn div(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_div256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.div(a, b)
        }
    }

    fn rem(&self, a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_rem256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
        {
            self.native_uint256_ops.rem(a, b)
        }
    }

    // fn leading_zeros(&self, a: [u64; 4]) -> u32 {
    //     ethereum_types::U256(a).leading_zeros()
    // }

    // fn bits(&self, a: [u64; 4]) -> usize {
    //     ethereum_types::U256(a).bits()
    // }

    // fn bit(&self, a: [u64; 4], index: usize) -> bool {
    //     ethereum_types::U256(a).bit(index)
    // }

    // fn byte(&self, a: [u64; 4], index: usize) -> u8 {
    //     ethereum_types::U256(a).byte(index)
    // }

    // ── U256 byte conversion ────────────────────────────────────────

    // fn to_big_endian(&self, a: [u64; 4]) -> [u8; 32] {
    //     ethereum_types::U256(a).to_big_endian()
    // }

    // fn from_big_endian(&self, bytes: &[u8]) -> [u64; 4] {
    //     ethereum_types::U256::from_big_endian(bytes).0
    // }

    // fn from_little_endian(&self, bytes: &[u8]) -> [u64; 4] {
    //     ethereum_types::U256::from_little_endian(bytes).0
    // }

    // ── U256 string parsing ─────────────────────────────────────────

    // fn from_dec_str(&self, s: &str) -> Result<[u64; 4], ParseU256Error> {
    //     ethereum_types::U256::from_dec_str(s)
    //         .map(|v| v.0)
    //         .map_err(|e| ParseU256Error(e.to_string()))
    // }

    // fn from_str_radix(&self, s: &str, radix: u32) -> Result<[u64; 4], ParseU256Error> {
    //     ethereum_types::U256::from_str_radix(s, radix)
    //         .map(|v| v.0)
    //         .map_err(|_| ParseU256Error("invalid string for radix".to_string()))
    // }

    // ── U512 operations (for ADDMOD) ────────────────────────────────

    // fn u512_from_u256(&self, a: [u64; 4]) -> [u64; 8] {
    //     let v = ethereum_types::U512::from(ethereum_types::U256(a));
    //     v.0
    // }

    // fn u512_overflowing_add(&self, a: [u64; 8], b: [u64; 8]) -> ([u64; 8], bool) {
    //     let a = ethereum_types::U512(a);
    //     let b = ethereum_types::U512(b);
    //     let (v, o) = a.overflowing_add(b);
    //     (v.0, o)
    // }

    // fn u512_rem(&self, a: [u64; 8], b: [u64; 8]) -> [u64; 8] {
    //     let a = ethereum_types::U512(a);
    //     let b = ethereum_types::U512(b);
    //     (a % b).0
    // }

    // fn u512_rem_u256(&self, a: [u64; 8], b: [u64; 4]) -> [u64; 8] {
    //     let a = ethereum_types::U512(a);
    //     let b = ethereum_types::U512::from(ethereum_types::U256(b));
    //     (a % b).0
    // }
}

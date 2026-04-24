use tiny_keccak::{Hasher, Keccak};

use alloy_consensus::crypto::{CryptoProvider, RecoveryError};
use alloy_primitives::Address;

use revm::precompile::{
    bls12_381::{G1Point, G1PointScalar, G2Point, G2PointScalar},
    Crypto, PrecompileError,
};

use guest_common::ffi::*;

use super::CustomEvmCrypto;

impl Crypto for CustomEvmCrypto {
    #[inline]
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        let mut output = zkvm_sha256_hash { data: [0u8; 32] };
        unsafe {
            zkvm_sha256(input.as_ptr(), input.len(), &mut output);
        }
        return output.data;
    }

    #[inline]
    fn blake2_compress(&self, rounds: u32, h: &mut [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) {
        unsafe {
            zkvm_blake2f(
                rounds,
                h.as_mut_ptr() as *mut zkvm_blake2f_state,
                m.as_ptr() as *const zkvm_blake2f_message,
                t.as_ptr() as *const zkvm_blake2f_offset,
                f as u8,
            );
        }
    }

    #[inline]
    fn ripemd160(&self, input: &[u8]) -> [u8; 32] {
        let mut output = zkvm_ripemd160_hash { data: [0u8; 32] };
        unsafe {
            zkvm_ripemd160(input.as_ptr(), input.len(), &mut output);
        }
        return output.data;
    }

    #[inline]
    fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, PrecompileError> {
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
        Ok(result)
    }

    #[inline]
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileError> {
        let mut pubkey_out = zkvm_secp256k1_pubkey { data: [0u8; 64] };
        let ret = unsafe {
            zkvm_secp256k1_ecrecover(
                msg.as_ptr() as *const zkvm_secp256k1_hash,
                sig.as_ptr() as *const zkvm_secp256k1_signature,
                recid,
                &mut pubkey_out,
            )
        };
        match ret {
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
            _ => Err(PrecompileError::Secp256k1RecoverFailed),
        }
    }

    /// secp256r1 (P-256) signature verification.
    #[inline]
    fn secp256r1_verify_signature(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
        let mut verified = false;
        unsafe {
            zkvm_secp256r1_verify(
                msg.as_ptr() as *const zkvm_secp256r1_hash,
                sig.as_ptr() as *const zkvm_secp256r1_signature,
                pk.as_ptr() as *const zkvm_secp256r1_pubkey,
                &mut verified,
            );
        }
        verified
    }

    #[inline]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let mut result = zkvm_bn254_g1_point { data: [0u8; 64] };
        let ret = unsafe {
            zkvm_bn254_g1_add(
                p1.as_ptr() as *const zkvm_bn254_g1_point,
                p2.as_ptr() as *const zkvm_bn254_g1_point,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::other("bn254_g1_add failed")),
        }
    }

    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let mut result = zkvm_bn254_g1_point { data: [0u8; 64] };
        let ret = unsafe {
            zkvm_bn254_g1_mul(
                point.as_ptr() as *const zkvm_bn254_g1_point,
                scalar.as_ptr() as *const zkvm_bn254_scalar,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::other("bn254_g1_mul failed")),
        }
    }

    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        // Each pair is G1 (64 bytes) + G2 (128 bytes) = 192 bytes laid out contiguously.
        let mut pairs_bytes: Vec<u8> = Vec::with_capacity(pairs.len() * 192);
        for (g1, g2) in pairs {
            pairs_bytes.extend_from_slice(g1);
            pairs_bytes.extend_from_slice(g2);
        }
        let mut verified = false;
        let ret = unsafe {
            zkvm_bn254_pairing(
                pairs_bytes.as_ptr() as *const zkvm_bn254_pairing_pair,
                pairs.len(),
                &mut verified,
            )
        };
        match ret {
            0 => Ok(verified),
            _ => Err(PrecompileError::other("bn254_pairing_check failed")),
        }
    }

    /// BLS12-381 G1 addition (returns 96-byte unpadded G1 point)
    fn bls12_381_g1_add(&self, a: G1Point, b: G1Point) -> Result<[u8; 96], PrecompileError> {
        // G1Point is ([u8; 48], [u8; 48])
        let mut a_bytes = [0u8; 96];
        a_bytes[..48].copy_from_slice(&a.0);
        a_bytes[48..].copy_from_slice(&a.1);
        let mut b_bytes = [0u8; 96];
        b_bytes[..48].copy_from_slice(&b.0);
        b_bytes[48..].copy_from_slice(&b.1);
        let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
        let ret = unsafe {
            zkvm_bls12_g1_add(
                a_bytes.as_ptr() as *const zkvm_bls12_381_g1_point,
                b_bytes.as_ptr() as *const zkvm_bls12_381_g1_point,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::Bls12381G1NotOnCurve),
        }
    }

    /// BLS12-381 G1 multi-scalar multiplication (returns 96-byte unpadded G1 point)
    fn bls12_381_g1_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G1PointScalar, PrecompileError>>,
    ) -> Result<[u8; 96], PrecompileError> {
        // G1PointScalar is (G1Point, [u8; 32]) = (([u8; 48], [u8; 48]), [u8; 32])
        // Each pair is laid out as: point (96 bytes) || scalar (32 bytes) = 128 bytes.
        let mut pairs_bytes: Vec<u8> = Vec::new();
        let mut num_pairs = 0usize;
        for pair in pairs {
            let (point, scalar) = pair?;
            pairs_bytes.extend_from_slice(&point.0);
            pairs_bytes.extend_from_slice(&point.1);
            pairs_bytes.extend_from_slice(&scalar);
            num_pairs += 1;
        }
        let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
        let ret = unsafe {
            zkvm_bls12_g1_msm(
                pairs_bytes.as_ptr() as *const zkvm_bls12_381_g1_msm_pair,
                num_pairs,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::Bls12381G1NotOnCurve),
        }
    }

    /// BLS12-381 G2 addition (returns 192-byte unpadded G2 point)
    fn bls12_381_g2_add(&self, a: G2Point, b: G2Point) -> Result<[u8; 192], PrecompileError> {
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
        let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
        let ret = unsafe {
            zkvm_bls12_g2_add(
                a_bytes.as_ptr() as *const zkvm_bls12_381_g2_point,
                b_bytes.as_ptr() as *const zkvm_bls12_381_g2_point,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::Bls12381G2NotOnCurve),
        }
    }

    /// BLS12-381 G2 multi-scalar multiplication (returns 192-byte unpadded G2 point)
    fn bls12_381_g2_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G2PointScalar, PrecompileError>>,
    ) -> Result<[u8; 192], PrecompileError> {
        // G2PointScalar is (G2Point, [u8; 32]) = (([u8; 48]*4), [u8; 32])
        // Each pair is laid out as: point (192 bytes) || scalar (32 bytes) = 224 bytes.
        let mut pairs_bytes: Vec<u8> = Vec::new();
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
        let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
        let ret = unsafe {
            zkvm_bls12_g2_msm(
                pairs_bytes.as_ptr() as *const zkvm_bls12_381_g2_msm_pair,
                num_pairs,
                &mut result,
            )
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::Bls12381G2NotOnCurve),
        }
    }

    /// BLS12-381 pairing check.
    fn bls12_381_pairing_check(
        &self,
        pairs: &[(G1Point, G2Point)],
    ) -> Result<bool, PrecompileError> {
        // Each pair is G1 (96 bytes) || G2 (192 bytes) = 288 bytes laid out contiguously.
        let mut pairs_bytes: Vec<u8> = Vec::with_capacity(pairs.len() * 288);
        for (g1, g2) in pairs {
            pairs_bytes.extend_from_slice(&g1.0);
            pairs_bytes.extend_from_slice(&g1.1);
            pairs_bytes.extend_from_slice(&g2.0);
            pairs_bytes.extend_from_slice(&g2.1);
            pairs_bytes.extend_from_slice(&g2.2);
            pairs_bytes.extend_from_slice(&g2.3);
        }
        let mut verified = false;
        let ret = unsafe {
            zkvm_bls12_pairing(
                pairs_bytes.as_ptr() as *const zkvm_bls12_381_pairing_pair,
                pairs.len(),
                &mut verified,
            )
        };
        match ret {
            0 => Ok(verified),
            _ => Err(PrecompileError::Bls12381G1NotOnCurve),
        }
    }

    /// BLS12-381 map field element to G1.
    fn bls12_381_fp_to_g1(&self, fp: &[u8; 48]) -> Result<[u8; 96], PrecompileError> {
        let mut result = zkvm_bls12_381_g1_point { data: [0u8; 96] };
        let ret = unsafe {
            zkvm_bls12_map_fp_to_g1(fp.as_ptr() as *const zkvm_bls12_381_fp, &mut result)
        };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::other("bls12_381_fp_to_g1 failed")),
        }
    }

    /// BLS12-381 map field element to G2.
    fn bls12_381_fp2_to_g2(&self, fp2: ([u8; 48], [u8; 48])) -> Result<[u8; 192], PrecompileError> {
        let mut fp2_bytes = [0u8; 96];
        fp2_bytes[..48].copy_from_slice(&fp2.0);
        fp2_bytes[48..].copy_from_slice(&fp2.1);
        let fp2_struct = zkvm_bls12_381_fp2 { data: fp2_bytes };
        let mut result = zkvm_bls12_381_g2_point { data: [0u8; 192] };
        let ret = unsafe { zkvm_bls12_map_fp2_to_g2(&fp2_struct, &mut result) };
        match ret {
            0 => Ok(result.data),
            _ => Err(PrecompileError::other("bls12_381_fp2_to_g2 failed")),
        }
    }

    /// KZG point evaluation.
    #[inline]
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileError> {
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
        if verified {
            Ok(())
        } else {
            Err(PrecompileError::BlobVerifyKzgProofFailed)
        }
    }
}

impl CryptoProvider for CustomEvmCrypto {
    /// Recover signer from signature and message hash, without ensuring low S values.
    fn recover_signer_unchecked(
        &self,
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        let sig_bytes: [u8; 64] = sig[..64].try_into().unwrap();
        let recid = sig[64];
        let mut pubkey_out = zkvm_secp256k1_pubkey { data: [0u8; 64] };
        let ret = unsafe {
            zkvm_secp256k1_ecrecover(
                msg.as_ptr() as *const zkvm_secp256k1_hash,
                sig_bytes.as_ptr() as *const zkvm_secp256k1_signature,
                recid,
                &mut pubkey_out,
            )
        };
        match ret {
            0 => {
                let mut hasher = Keccak::v256();
                hasher.update(&pubkey_out.data);
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                Ok(Address::from_slice(&hash[12..]))
            }
            _ => Err(RecoveryError::new()),
        }
    }

    /// Verify a signature against a public key and message hash, without ensuring low S values.
    fn verify_and_compute_signer_unchecked(
        &self,
        pubkey: &[u8; 65],
        sig: &[u8; 64],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        // pubkey is 65 bytes: prefix + 64 bytes (x || y)
        let pk_bytes: &[u8; 64] = pubkey[1..].try_into().unwrap();
        let mut verified = false;
        let ret = unsafe {
            zkvm_secp256k1_verify(
                msg.as_ptr() as *const zkvm_secp256k1_hash,
                sig.as_ptr() as *const zkvm_secp256k1_signature,
                pk_bytes.as_ptr() as *const zkvm_secp256k1_pubkey,
                &mut verified,
            )
        };
        match (ret, verified) {
            (0, true) => {
                let mut hasher = Keccak::v256();
                hasher.update(pk_bytes);
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);
                Ok(Address::from_slice(&hash[12..]))
            }
            _ => Err(RecoveryError::new()),
        }
    }
}

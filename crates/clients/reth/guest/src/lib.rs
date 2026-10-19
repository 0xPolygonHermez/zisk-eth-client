//! ZisK guest for the reth stateless validator.
//!
//! The guest speaks the execution-specs stateless format
//! (tests-zkevm v21.0.1): the input is a 2-byte big-endian schema id (fork and
//! revision) followed by the SSZ-encoded [`StatelessInput`], and the output is
//! the plain SSZ [`StatelessValidationResult`] (43 bytes). Validation itself is
//! `stateless_validator_reth::guest::run_stateless_guest`; this crate adds the
//! ZisK crypto provider, I/O, and logging around it.

mod crypto;
mod run;
mod summary;

pub use crypto::CustomEvmCrypto;
pub use run::*;
pub use summary::*;

pub use stateless_validator_common::guest::{
    input::{ExecutionWitness, ProtocolFork},
    StatelessInput, StatelessValidationResult,
};
pub use stateless_validator_common::{SszDecode, SszEncode};

/// Run the stateless validator on stateless input bytes and return the
/// output bytes. Never panics on invalid input: a malformed input or
/// a failed validation yields a result with `successful_validation == false`.
pub fn validate(input: &[u8]) -> Vec<u8> {
    stateless_validator_reth::guest::run_stateless_guest(input)
}

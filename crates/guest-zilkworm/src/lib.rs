//! Zilkworm C++ ZKEVM integration — host-side glue.
//!
//! - [`run`] drives zilkworm's `prover/host_lib` FFI (`z6m_run`), so the
//!   native path matches the in-zkVM guest exactly. Hints emitted during the
//!   C++ EVM execution flow through ziskos's `extern "C"` symbols.
//! - [`fetch_block_and_witness`] + [`FetchRequest`] are re-exported from
//!   `z6m_common` for input building.
//! - [`ELF`] is the prebuilt guest ELF, embedded at build time.

use zisk_sdk::{load_program, GuestProgram};

pub use z6m_common::{fetch_block_and_witness, FetchRequest};


pub const ELF: GuestProgram = load_program!("z6m_guest");

unsafe extern "C" {
    fn z6m_run();
}

pub fn run() {
    unsafe { z6m_run() }
}

//! ziskethone C++ EVM integration — host-side glue.
//!
//! [`ELF`] is the prebuilt guest ELF, embedded at build time; the
//! validator/proving path loads it through the embedded ZisK SDK.
//!
//! [`run`] executes ziskethone's C++ block validation natively, in-process,
//! via the `zeg_run` FFI entry point (zilkworm-style) — the same pipeline the
//! ZisK ELF runs, but on the host x86 EVM for fast input checking.

use zisk_sdk::{load_program, GuestProgram};

pub const ELF: GuestProgram = load_program!("zisk_eth_guest");

unsafe extern "C" {
    fn zeg_run(input: *const u8, len: usize, out: *mut u8) -> i32;
}

/// Run ziskethone's C++ block validation in-process on a ZEG0 container.
/// Returns the 32-byte execution block hash. Panics on a nonzero FFI status.
pub fn run(input: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let rc = unsafe { zeg_run(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    assert!(rc == 0, "ziskethone zeg_run failed with status {rc}");
    out
}

//! ziskethone C++ EVM integration — host-side glue.
//!
//! [`ELF`] is the prebuilt guest ELF, committed to the repo at `elf/` and
//! embedded at compile time — building it requires the xPack RISC-V toolchain,
//! but *consuming* it does not, so proving works with nothing installed. To
//! regenerate the committed ELF after changing the C++ guest, build with the
//! `rebuild-guest` feature (see build.rs).
//!
//! [`run`] executes ziskethone's C++ block validation natively, in-process, via
//! the `zeg_run` FFI (zilkworm-style) — the same pipeline the ZisK ELF runs, but
//! on the host CPU for fast input checking. Its native lib is compiled from the
//! ziskethone submodule with a normal C++ toolchain (not the RISC-V one).

use zisk_sdk::{load_program, GuestProgram};

/// The committed guest ELF, embedded (and hashed) at compile time from the
/// checked-in `elf/zisk_eth_guest.elf`. No RISC-V toolchain needed to use it.
pub const ELF: GuestProgram = load_program!("zisk_eth_guest", "elf/zisk_eth_guest.elf");

/// Native `zeg_run` FFI ([`run`]). Behind the `ziskethone` feature because it
/// compiles+links the C++ evmone library (needs clang >= 15 / g++ >= 12). The
/// ELF above needs none of that, so proving-only consumers skip this entirely.
#[cfg(feature = "ziskethone")]
mod ffi {
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
}

#[cfg(feature = "ziskethone")]
pub use ffi::run;

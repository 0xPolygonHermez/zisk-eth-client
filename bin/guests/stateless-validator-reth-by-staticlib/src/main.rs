// lib_impl.rs is the canonical source; it is also compiled separately as a
// staticlib for RISC-V explicit linking. The binary includes it directly so
// that the #[no_mangle] main symbol is available without an rlib dependency.
#![no_main]
#![cfg_attr(zisk_guest, no_std)]
include!("lib_impl.rs");

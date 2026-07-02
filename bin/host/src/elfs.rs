use zisk_sdk::{GuestProgram, load_program};

// ELF binaries for the host to load into the zkVM
pub const ELF_RETH: GuestProgram = load_program!("zec-reth");
pub const ELF_ETHREX: GuestProgram = load_program!("zec-ethrex");
pub use guest_ziskethone::ELF as ELF_ZISKETHONE;
// Add more ELF binaries here as needed — see docs/adding-a-client.md

use zisk_sdk::{GuestProgram, load_program};

// ELF binaries for the host to load into the zkVM
pub(crate) const ELF_RETH: GuestProgram = load_program!("zec-reth");
pub(crate) const ELF_ETHREX: GuestProgram = load_program!("zec-ethrex");
// Add more ELF binaries here as needed

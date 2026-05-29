use zisk_sdk::{load_program, GuestProgram};

// ELF binaries for the host to load into the zkVM
pub const ELF_RETH: GuestProgram = load_program!("zec-reth");
pub const ELF_ETHREX: GuestProgram = load_program!("zec-ethrex");
pub use guest_zilkworm::ELF as ELF_ZILKWORM;
// Add more ELF binaries here as needed — see docs/adding-a-client.md

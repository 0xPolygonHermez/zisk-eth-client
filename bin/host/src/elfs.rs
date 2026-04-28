use zisk_sdk::{ElfBinary, include_elf};

// ELF binaries for the host to load into the zkVM
pub(crate) const ELF_RETH: ElfBinary = include_elf!("zec-reth");
pub(crate) const ELF_ETHREX: ElfBinary = include_elf!("zec-ethrex");
// Add more ELF binaries here as needed

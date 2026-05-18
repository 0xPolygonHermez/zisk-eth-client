use zisk_sdk::{load_program, GuestProgram};

pub const ELF_BLS12_381: GuestProgram = load_program!("bls12_381");
pub const ELF_BN254: GuestProgram = load_program!("bn254");
pub const ELF_HASHES: GuestProgram = load_program!("hashes");
pub const ELF_MODEXP: GuestProgram = load_program!("modexp");
pub const ELF_OPCODES: GuestProgram = load_program!("opcodes");
pub const ELF_SECP256K1: GuestProgram = load_program!("secp256k1");
pub const ELF_SECP256R1: GuestProgram = load_program!("secp256r1");

#[cfg(test)]
mod execute {
    use super::*;
    use zisk_sdk::{run, ZiskStdin};

    // TODO: Finish
    // #[test]
    // fn execute_opcodes() {
    //     run(&ELF_OPCODES, ZiskStdin::new(), None).expect("opcodes guest emulation failed");
    // }

    #[test]
    fn execute_hashes() {
        run(&ELF_HASHES, ZiskStdin::new(), None).expect("hashes guest emulation failed");
    }

    #[test]
    fn execute_modexp() {
        run(&ELF_MODEXP, ZiskStdin::new(), None).expect("modexp guest emulation failed");
    }

    #[test]
    fn execute_secp256k1() {
        run(&ELF_SECP256K1, ZiskStdin::new(), None).expect("secp256k1 guest emulation failed");
    }

    #[test]
    fn execute_secp256r1() {
        run(&ELF_SECP256R1, ZiskStdin::new(), None).expect("secp256r1 guest emulation failed");
    }

    #[test]
    fn execute_bls12_381() {
        run(&ELF_BLS12_381, ZiskStdin::new(), None).expect("bls12_381 guest emulation failed");
    }

    #[test]
    fn execute_bn254() {
        run(&ELF_BN254, ZiskStdin::new(), None).expect("bn254 guest emulation failed");
    }
}

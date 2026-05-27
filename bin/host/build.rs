use zisk_sdk::build_program;

fn main() {
    // Rust guests: cargo-zisk builds them and sets ZISK_ELF_<name> / ZISK_ELF_HASH_<name>.
    build_program("../guests/stateless-validator-reth");
    build_program("../guests/stateless-validator-ethrex");
}

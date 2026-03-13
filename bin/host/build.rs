use zisk_sdk::build_program;

fn main() {
    // Build the guest programs
    build_program("../guests/stateless-validator-reth");
    build_program("../guests/stateless-validator-ethrex");
}

/// Map a chain ID to a display name. Returns `"Unknown"` for unsupported chains.
pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Mainnet",
        11155111 => "Sepolia",
        17000 => "Holesky",
        560048 => "Hoodi",
        _ => "Unknown",
    }
}

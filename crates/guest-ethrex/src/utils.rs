use ethrex_common::types::Block;

/// Get chain name from chain ID
pub fn get_chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Mainnet",
        11155111 => "Sepolia",
        17000 => "Holesky",
        560048 => "Hoodi",
        _ => "Unknown",
    }
}

/// Extract common execution payload information across forks.
pub fn extract_block_info(block: &Block) -> (u64, u64, usize) {
    let block_number = block.header.number;
    let gas_used = block.header.gas_used;
    let tx_count = block.body.transactions.len();

    (block_number, gas_used, tx_count)
}

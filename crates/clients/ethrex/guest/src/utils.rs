use ethrex_common::types::Block;

/// Extract common execution payload information across forks.
pub fn extract_block_info(block: &Block) -> (u64, u64, usize) {
    let block_number = block.header.number;
    let gas_used = block.header.gas_used;
    let tx_count = block.body.transactions.len();

    (block_number, gas_used, tx_count)
}

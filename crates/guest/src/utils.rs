use std::sync::Arc;

use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::Block;

/// Get chain spec from chain ID
pub fn get_chain_spec(chain_id: u64) -> Arc<ChainSpec> {
    ChainSpec::from_chain_id(chain_id).unwrap_or_else(|| {
        panic!(
            "Unsupported chain ID: {}. Please add it to the chain spec mapping.",
            chain_id
        )
    })
}

/// Get chain name from chain ID
pub fn get_chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        0x1 => "Mainnet",
        0xaa36a7 => "Sepolia",
        0x4268 => "Holesky",
        0x5 => "Goerli",
        _ => "Unknown",
        // Add more chain IDs as needed
    }
}

/// Extract common execution payload information across forks.
pub fn extract_block_info(block: &Block) -> (u64, u64, usize) {
    let block_number = block.header.number;
    let gas_used = block.header.gas_used;
    let tx_count = block.body.transactions.len();

    (block_number, gas_used, tx_count)
}

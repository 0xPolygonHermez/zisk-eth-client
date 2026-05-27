use std::sync::Arc;

use alloy_genesis::{ChainConfig, Genesis};
use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::Block;

/// Get chain spec from chain config
pub fn get_chain_spec(chain_config: &ChainConfig) -> Arc<ChainSpec> {
    let genesis = Genesis {
        config: chain_config.clone(),
        ..Default::default()
    };

    Arc::new(ChainSpec::from_genesis(genesis))
}

/// Extract common execution payload information across forks.
pub fn extract_block_info(block: &Block) -> (u64, u64, usize) {
    let block_number = block.header.number;
    let gas_used = block.header.gas_used;
    let tx_count = block.body.transactions.len();

    (block_number, gas_used, tx_count)
}

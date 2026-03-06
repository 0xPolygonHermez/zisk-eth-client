use std::sync::Arc;

use reth_chainspec::ChainSpec;
use stateless_validator_common::new_payload_request::NewPayloadRequest;

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
pub fn extract_block_info(req: &NewPayloadRequest) -> (u64, u64, usize) {
    match req {
        NewPayloadRequest::Bellatrix(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::Capella(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::Deneb(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
        NewPayloadRequest::ElectraFulu(r) => (
            r.execution_payload.block_number,
            r.execution_payload.gas_used,
            r.execution_payload.transactions.len(),
        ),
    }
}

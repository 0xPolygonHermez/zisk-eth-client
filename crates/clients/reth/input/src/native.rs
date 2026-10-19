//! Native (host-side) stateless execution of a block.
//!
//! Runs the same validation the guest runs, on the host, to recover what the
//! stateless input needs but the execution-layer RPC does not provide: the
//! EIP-7685 execution requests (and, as a fallback, the EIP-7928 block access
//! list). A block that fails here would fail in the guest too, so this doubles
//! as an input check.

use std::{collections::BTreeMap, sync::Arc};

use alloy_consensus::{Block, TxEnvelope};
use alloy_eips::{
    eip2718::{Decodable2718, Encodable2718},
    eip7840::BlobParams,
};
use alloy_genesis::{ChainConfig, Genesis};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_rpc_types_debug::ExecutionWitness;
use anyhow::{bail, ensure, Context, Result};
use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm_ethereum::EthEvmConfig;
use stateless_tries::zeth::SparseState;
use stateless_validator_common::{guest::input::ProtocolFork, SszDecode};

use crate::block_input::{
    BuilderDepositRequest, BuilderExitRequest, ConsolidationRequest, DepositRequest,
    ExecutionRequests, WithdrawalRequest,
};

/// What native execution recovers for the stateless input.
#[derive(Debug, Clone)]
pub struct NativeExecution {
    pub requests: ExecutionRequests,
    /// Raw RLP block access list, present from Amsterdam on.
    pub block_access_list: Option<Bytes>,
}

/// Execute `block` statelessly against `witness` and return its requests and
/// block access list. Fails if the block is invalid.
pub fn execute_block(
    chain_id: u64,
    fork: ProtocolFork,
    block: &Block<TxEnvelope>,
    witness: &ExecutionWitness,
) -> Result<NativeExecution> {
    let chain_spec = Arc::new(ChainSpec::from(Genesis {
        config: chain_config(fork, chain_id),
        ..Default::default()
    }));
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    // reth's transaction type is the alloy envelope without the blob sidecar
    // variant; round-trip through the 2718 encoding to convert.
    let transactions = block
        .body
        .transactions
        .iter()
        .enumerate()
        .map(|(i, tx)| {
            TransactionSigned::decode_2718(&mut tx.encoded_2718().as_slice())
                .with_context(|| format!("re-decoding transaction #{i}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let reth_block = reth_ethereum_primitives::Block {
        header: block.header.clone(),
        body: alloy_consensus::BlockBody {
            transactions,
            ommers: block.body.ommers.clone(),
            withdrawals: block.body.withdrawals.clone(),
        },
    };

    let output = stateless::stateless_validation_with_trie::<SparseState, _, _>(
        reth_block,
        witness.clone(),
        chain_spec,
        evm_config,
    )
    .map_err(|err| anyhow::anyhow!("native stateless validation failed: {err}"))?;

    let requests = parse_requests(output.execution_output.result.requests.iter())?;
    let block_access_list = output
        .block_access_list
        .map(|bal| Bytes::from(alloy_rlp::encode(&bal)));
    Ok(NativeExecution {
        requests,
        block_access_list,
    })
}

/// Decode EIP-7685 `type || records` blobs into the SSZ request lists.
fn parse_requests<'a>(blobs: impl Iterator<Item = &'a Bytes>) -> Result<ExecutionRequests> {
    let mut requests = ExecutionRequests::default();
    for blob in blobs {
        let (ty, data) = blob.split_first().context("empty execution request blob")?;
        match ty {
            0x00 => requests.deposits = records::<DepositRequest>(data, "deposit")?.into(),
            0x01 => requests.withdrawals = records::<WithdrawalRequest>(data, "withdrawal")?.into(),
            0x02 => {
                requests.consolidations =
                    records::<ConsolidationRequest>(data, "consolidation")?.into()
            }
            0x03 => {
                requests.builder_deposits =
                    records::<BuilderDepositRequest>(data, "builder deposit")?.into()
            }
            0x04 => {
                requests.builder_exits = records::<BuilderExitRequest>(data, "builder exit")?.into()
            }
            other => bail!("unknown execution request type {other:#04x}"),
        }
    }
    Ok(requests)
}

fn records<T: SszDecode>(data: &[u8], label: &str) -> Result<Vec<T>> {
    ensure!(
        T::is_fixed_size(),
        "{label} request record is not fixed size"
    );
    let size = T::fixed_size();
    ensure!(
        !data.is_empty() && data.len().is_multiple_of(size),
        "{label} request data length {} is not a multiple of {size}",
        data.len()
    );
    data.chunks(size)
        .map(|chunk| {
            T::from_ssz_bytes(chunk)
                .map_err(|err| anyhow::anyhow!("invalid {label} request record: {err:?}"))
        })
        .collect()
}

/// The chain configuration the guest derives from the schema fork and chain
/// id (every fork active at genesis), plus the network's deposit contract,
/// which upstream hardcodes to mainnet's.
pub fn chain_config(fork: ProtocolFork, chain_id: u64) -> ChainConfig {
    let time_at = |target| (fork >= target).then_some(0);
    ChainConfig {
        chain_id,
        homestead_block: Some(0),
        dao_fork_block: Some(0),
        dao_fork_support: true,
        eip150_block: Some(0),
        eip155_block: Some(0),
        eip158_block: Some(0),
        byzantium_block: Some(0),
        constantinople_block: Some(0),
        petersburg_block: Some(0),
        istanbul_block: Some(0),
        muir_glacier_block: Some(0),
        berlin_block: Some(0),
        london_block: Some(0),
        arrow_glacier_block: Some(0),
        gray_glacier_block: Some(0),
        merge_netsplit_block: Some(0),
        shanghai_time: time_at(ProtocolFork::Shanghai),
        cancun_time: time_at(ProtocolFork::Cancun),
        prague_time: time_at(ProtocolFork::Prague),
        osaka_time: time_at(ProtocolFork::Osaka),
        bpo1_time: time_at(ProtocolFork::BPO1),
        bpo2_time: time_at(ProtocolFork::BPO2),
        amsterdam_time: time_at(ProtocolFork::Amsterdam),
        terminal_total_difficulty: Some(U256::ZERO),
        terminal_total_difficulty_passed: true,
        blob_schedule: blob_schedule(fork),
        deposit_contract_address: Some(deposit_contract(chain_id)),
        ..Default::default()
    }
}

/// EIP-6110 deposit contract per network.
pub fn deposit_contract(chain_id: u64) -> Address {
    match chain_id {
        11155111 => address!("0x7f02c3e3c98b133055b8b348b2ac625669ed295d"),
        17000 => address!("0x4242424242424242424242424242424242424242"),
        // Mainnet and Hoodi.
        _ => alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS,
    }
}

fn blob_schedule(fork: ProtocolFork) -> BTreeMap<String, BlobParams> {
    let (key, params) = match fork {
        ProtocolFork::Cancun => ("cancun", BlobParams::cancun()),
        ProtocolFork::Prague => ("prague", BlobParams::prague()),
        ProtocolFork::Osaka => ("osaka", BlobParams::osaka()),
        ProtocolFork::BPO1 => ("bpo1", BlobParams::bpo1()),
        ProtocolFork::BPO2 => ("bpo2", BlobParams::bpo2()),
        // alloy-genesis spells this key with a capital letter.
        ProtocolFork::Amsterdam => ("Amsterdam", BlobParams::bpo2()),
        _ => return BTreeMap::new(),
    };
    BTreeMap::from([(key.to_string(), params)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use stateless_validator_common::SszEncode;

    #[test]
    fn parses_typed_request_blobs() {
        let deposit = DepositRequest {
            pubkey: [1; 48],
            withdrawal_credentials: [2; 32],
            amount: 32_000_000_000,
            signature: [3; 96],
            index: 7,
        };
        let withdrawal = WithdrawalRequest {
            source_address: [4; 20],
            validator_pubkey: [5; 48],
            amount: 1,
        };
        let mut deposits = vec![0x00];
        deposits.extend(deposit.to_ssz());
        deposits.extend(deposit.to_ssz());
        let mut withdrawals = vec![0x01];
        withdrawals.extend(withdrawal.to_ssz());
        let blobs = [Bytes::from(deposits), Bytes::from(withdrawals)];

        let requests = parse_requests(blobs.iter()).unwrap();
        assert_eq!(requests.deposits.len(), 2);
        assert_eq!(requests.deposits[1], deposit);
        assert_eq!(requests.withdrawals.len(), 1);
        assert_eq!(requests.withdrawals[0], withdrawal);
        assert!(requests.consolidations.is_empty());
    }

    #[test]
    fn rejects_malformed_blobs() {
        assert!(parse_requests([Bytes::from(vec![0x00, 1, 2, 3])].iter()).is_err());
        assert!(parse_requests([Bytes::from(vec![0x09])].iter()).is_err());
    }

    #[test]
    fn deposit_contract_per_network() {
        assert_eq!(
            deposit_contract(11155111),
            address!("0x7f02c3e3c98b133055b8b348b2ac625669ed295d")
        );
        assert_eq!(
            deposit_contract(1),
            alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS
        );
        assert_eq!(deposit_contract(560048), deposit_contract(1));
        assert_eq!(
            chain_config(ProtocolFork::Amsterdam, 11155111).amsterdam_time,
            Some(0)
        );
        assert_eq!(chain_config(ProtocolFork::BPO2, 1).amsterdam_time, None);
    }
}

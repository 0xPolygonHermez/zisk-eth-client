//! Block input construction.
//!
//! Builds the execution-specs (tests-zkevm v21.0.1) stateless input for a
//! block from consensus-layer-independent data: the block itself, its execution
//! witness, and (from Prague on) the EIP-7685 execution requests, which the
//! block does not carry and must come from a consensus-layer node. The schema
//! fork is derived from the chain id and the block timestamp.

use alloy_consensus::{Block, Header, Transaction as _, TxEnvelope};
use alloy_eips::eip2718::Encodable2718;
use alloy_hardforks::ethereum::*;
use alloy_primitives::{Bytes, B256, U256};
use alloy_rlp::Decodable;
use alloy_rpc_types_debug::ExecutionWitness as RpcExecutionWitness;
use anyhow::{bail, ensure, Context, Result};

use stateless_validator_common::{
    guest::{
        input::{
            new_payload_request::{
                ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ExecutionPayloadV4,
                ExecutionRequestsElectraFulu, ExtraData, NewPayloadRequest,
                NewPayloadRequestBellatrix, NewPayloadRequestCapella, NewPayloadRequestDeneb,
                NewPayloadRequestElectraFulu, NewPayloadRequestGloas, Transactions,
                VersionedHashes, Withdrawal, Withdrawals,
            },
            ExecutionWitness, ProtocolFork, MAX_BYTES_PER_CODE, MAX_BYTES_PER_HEADER,
            MAX_BYTES_PER_WITNESS_NODE, MAX_WITNESS_HEADERS,
        },
        StatelessInput, StatelessValidationResult,
    },
    HashTreeRoot as _, ProgressiveList, Sha2Hasher, SszEncode as _, SszList,
};

pub use stateless_validator_common::guest::input::new_payload_request::{
    BuilderDepositRequest, BuilderExitRequest, ConsolidationRequest, DepositRequest,
    ExecutionRequestsGloas as ExecutionRequests, WithdrawalRequest,
};

/// A block input: the execution-specs stateless bytes the guest reads,
/// the output it must produce, and the block facts the host reports.
#[derive(Debug, Clone)]
pub struct BlockInput {
    pub fork: ProtocolFork,
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: B256,
    pub timestamp: u64,
    pub gas_used: u64,
    pub tx_count: usize,
    /// `schema_id || SSZ(StatelessInput)`, what the guest reads.
    pub stateless_input_bytes: Vec<u8>,
    /// Plain SSZ `StatelessValidationResult` the guest must write for a valid block.
    pub stateless_output_bytes: Vec<u8>,
}

/// Activation timestamps of the forks that matter for the input schema.
struct Schedule {
    paris: u64,
    shanghai: u64,
    cancun: u64,
    prague: u64,
    osaka: Option<u64>,
    bpo1: Option<u64>,
    bpo2: Option<u64>,
    amsterdam: Option<u64>,
}

fn schedule(chain_id: u64) -> Option<Schedule> {
    Some(match chain_id {
        1 => Schedule {
            paris: MAINNET_PARIS_TIMESTAMP,
            shanghai: MAINNET_SHANGHAI_TIMESTAMP,
            cancun: MAINNET_CANCUN_TIMESTAMP,
            prague: MAINNET_PRAGUE_TIMESTAMP,
            osaka: Some(MAINNET_OSAKA_TIMESTAMP),
            bpo1: Some(MAINNET_BPO1_TIMESTAMP),
            bpo2: Some(MAINNET_BPO2_TIMESTAMP),
            amsterdam: None,
        },
        11155111 => Schedule {
            paris: SEPOLIA_PARIS_TIMESTAMP,
            shanghai: SEPOLIA_SHANGHAI_TIMESTAMP,
            cancun: SEPOLIA_CANCUN_TIMESTAMP,
            prague: SEPOLIA_PRAGUE_TIMESTAMP,
            osaka: Some(SEPOLIA_OSAKA_TIMESTAMP),
            bpo1: Some(SEPOLIA_BPO1_TIMESTAMP),
            bpo2: Some(SEPOLIA_BPO2_TIMESTAMP),
            amsterdam: Some(SEPOLIA_AMSTERDAM_TIMESTAMP),
        },
        560048 => Schedule {
            // Hoodi launched post-Cancun.
            paris: 0,
            shanghai: 0,
            cancun: 0,
            prague: HOODI_PRAGUE_TIMESTAMP,
            osaka: Some(HOODI_OSAKA_TIMESTAMP),
            bpo1: Some(HOODI_BPO1_TIMESTAMP),
            bpo2: Some(HOODI_BPO2_TIMESTAMP),
            amsterdam: None,
        },
        17000 => Schedule {
            paris: HOLESKY_PARIS_TIMESTAMP,
            shanghai: HOLESKY_SHANGHAI_TIMESTAMP,
            cancun: HOLESKY_CANCUN_TIMESTAMP,
            prague: HOLESKY_PRAGUE_TIMESTAMP,
            osaka: Some(HOLESKY_OSAKA_TIMESTAMP),
            bpo1: Some(HOLESKY_BPO1_TIMESTAMP),
            bpo2: Some(HOLESKY_BPO2_TIMESTAMP),
            amsterdam: None,
        },
        _ => return None,
    })
}

/// The schema fork active on `chain_id` at `timestamp`.
pub fn fork_at(chain_id: u64, timestamp: u64) -> Result<ProtocolFork> {
    let s = schedule(chain_id)
        .with_context(|| format!("unsupported chain id {chain_id}: no fork schedule"))?;
    let active = |at: Option<u64>| at.is_some_and(|at| timestamp >= at);
    let fork = if active(s.amsterdam) {
        ProtocolFork::Amsterdam
    } else if active(s.bpo2) {
        ProtocolFork::BPO2
    } else if active(s.bpo1) {
        ProtocolFork::BPO1
    } else if active(s.osaka) {
        ProtocolFork::Osaka
    } else if timestamp >= s.prague {
        ProtocolFork::Prague
    } else if timestamp >= s.cancun {
        ProtocolFork::Cancun
    } else if timestamp >= s.shanghai {
        ProtocolFork::Shanghai
    } else if timestamp >= s.paris {
        ProtocolFork::Paris
    } else {
        bail!("timestamp {timestamp} on chain {chain_id} is pre-merge; the stateless schema starts at Paris");
    };
    Ok(fork)
}

/// Build the stateless input for `block`.
///
/// `block_access_list` is the raw EIP-7928 encoding and is required from
/// Amsterdam on; `requests` must be the block's EIP-7685 execution requests
/// (empty before Prague). The witness headers are sorted and checked to be
/// contiguous and to end with the parent header.
pub fn build_block_input(
    chain_id: u64,
    block: &Block<TxEnvelope>,
    block_access_list: Option<Bytes>,
    requests: ExecutionRequests,
    witness: &RpcExecutionWitness,
) -> Result<BlockInput> {
    let header = &block.header;
    let fork = fork_at(chain_id, header.timestamp)?;
    let block_hash = header.hash_slow();

    let new_payload_request = new_payload_request(fork, block, block_access_list, requests)
        .with_context(|| format!("block {} payload", header.number))?;
    let witness = convert_witness(witness, header.number)?;

    let new_payload_request_root = new_payload_request.hash_tree_root(&Sha2Hasher);
    let stateless_output_bytes = StatelessValidationResult {
        new_payload_request_root,
        successful_validation: true,
        chain_id,
        schema_id: fork.schema_id(),
    }
    .to_ssz();

    let stateless_input_bytes = StatelessInput {
        new_payload_request,
        witness,
        chain_id,
    }
    .to_schema_prefixed_ssz(fork);

    Ok(BlockInput {
        fork,
        chain_id,
        block_number: header.number,
        block_hash,
        timestamp: header.timestamp,
        gas_used: header.gas_used,
        tx_count: block.body.transactions.len(),
        stateless_input_bytes,
        stateless_output_bytes,
    })
}

/// Fill the fields every `ExecutionPayloadV*` shares, then the fork-specific ones.
macro_rules! payload {
    ($ty:ident, $header:expr, $txs:expr $(, $field:ident: $value:expr)* $(,)?) => {
        $ty {
            parent_hash: $header.parent_hash.0,
            fee_recipient: $header.beneficiary.into_array(),
            state_root: $header.state_root.0,
            receipts_root: $header.receipts_root.0,
            logs_bloom: $header.logs_bloom.0 .0,
            prev_randao: $header.mix_hash.0,
            block_number: $header.number,
            gas_limit: $header.gas_limit,
            gas_used: $header.gas_used,
            timestamp: $header.timestamp,
            extra_data: ExtraData::try_from($header.extra_data.to_vec())
                .map_err(|err| anyhow::anyhow!("extra_data exceeds the SSZ bound: {err:?}"))?,
            base_fee_per_gas: U256::from($header.base_fee_per_gas.unwrap_or_default())
                .to_le_bytes::<32>(),
            block_hash: $header.hash_slow().0,
            transactions: $txs,
            $($field: $value,)*
        }
    };
}

fn new_payload_request(
    fork: ProtocolFork,
    block: &Block<TxEnvelope>,
    block_access_list: Option<Bytes>,
    requests: ExecutionRequests,
) -> Result<NewPayloadRequest> {
    let header: &Header = &block.header;
    let txs = &block.body.transactions;

    let transactions = Transactions::from(
        txs.iter()
            .map(|tx| ProgressiveList::from(tx.encoded_2718()))
            .collect::<Vec<_>>(),
    );
    let withdrawals = || -> Result<Withdrawals> {
        let list = block
            .body
            .withdrawals
            .as_ref()
            .context("post-Shanghai block without withdrawals")?;
        Ok(Withdrawals::from(
            list.iter()
                .map(|w| Withdrawal {
                    index: w.index,
                    validator_index: w.validator_index,
                    address: w.address.into_array(),
                    amount: w.amount,
                })
                .collect::<Vec<_>>(),
        ))
    };
    let versioned_hashes = || {
        VersionedHashes::from(
            txs.iter()
                .flat_map(|tx| tx.blob_versioned_hashes().unwrap_or_default().to_vec())
                .map(|hash| hash.0)
                .collect::<Vec<_>>(),
        )
    };
    let blob_gas_used = || {
        header
            .blob_gas_used
            .context("post-Cancun block without blob_gas_used")
    };
    let excess_blob_gas = || {
        header
            .excess_blob_gas
            .context("post-Cancun block without excess_blob_gas")
    };
    let parent_beacon_block_root = || {
        header
            .parent_beacon_block_root
            .map(|root| root.0)
            .context("post-Cancun block without parent_beacon_block_root")
    };
    let no_requests = |requests: &ExecutionRequests| {
        requests.deposits.is_empty()
            && requests.withdrawals.is_empty()
            && requests.consolidations.is_empty()
            && requests.builder_deposits.is_empty()
            && requests.builder_exits.is_empty()
    };

    Ok(match fork {
        ProtocolFork::Paris => {
            ensure!(
                no_requests(&requests),
                "pre-Prague block with execution requests"
            );
            NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
                execution_payload: payload!(ExecutionPayloadV1, header, transactions),
            })
        }
        ProtocolFork::Shanghai => {
            ensure!(
                no_requests(&requests),
                "pre-Prague block with execution requests"
            );
            NewPayloadRequest::Capella(NewPayloadRequestCapella {
                execution_payload: payload!(
                    ExecutionPayloadV2,
                    header,
                    transactions,
                    withdrawals: withdrawals()?,
                ),
            })
        }
        ProtocolFork::Cancun => {
            ensure!(
                no_requests(&requests),
                "pre-Prague block with execution requests"
            );
            NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
                execution_payload: payload!(
                    ExecutionPayloadV3,
                    header,
                    transactions,
                    withdrawals: withdrawals()?,
                    blob_gas_used: blob_gas_used()?,
                    excess_blob_gas: excess_blob_gas()?,
                ),
                versioned_hashes: versioned_hashes(),
                parent_beacon_block_root: parent_beacon_block_root()?,
            })
        }
        ProtocolFork::Prague | ProtocolFork::Osaka | ProtocolFork::BPO1 | ProtocolFork::BPO2 => {
            ensure!(
                requests.builder_deposits.is_empty() && requests.builder_exits.is_empty(),
                "pre-Amsterdam block with builder requests"
            );
            NewPayloadRequest::ElectraFulu(NewPayloadRequestElectraFulu {
                execution_payload: payload!(
                    ExecutionPayloadV3,
                    header,
                    transactions,
                    withdrawals: withdrawals()?,
                    blob_gas_used: blob_gas_used()?,
                    excess_blob_gas: excess_blob_gas()?,
                ),
                versioned_hashes: versioned_hashes(),
                parent_beacon_block_root: parent_beacon_block_root()?,
                execution_requests: ExecutionRequestsElectraFulu {
                    deposits: requests.deposits,
                    withdrawals: requests.withdrawals,
                    consolidations: requests.consolidations,
                },
            })
        }
        ProtocolFork::Amsterdam => {
            let block_access_list = block_access_list
                .context("Amsterdam block without its block access list (EIP-7928)")?;
            let slot_number = header
                .slot_number
                .context("Amsterdam block without slot_number")?;
            NewPayloadRequest::Gloas(NewPayloadRequestGloas {
                execution_payload: payload!(
                    ExecutionPayloadV4,
                    header,
                    transactions,
                    withdrawals: withdrawals()?,
                    blob_gas_used: blob_gas_used()?,
                    excess_blob_gas: excess_blob_gas()?,
                    block_access_list: ProgressiveList::from(block_access_list.to_vec()),
                    slot_number: slot_number,
                ),
                versioned_hashes: versioned_hashes(),
                parent_beacon_block_root: parent_beacon_block_root()?,
                execution_requests: requests,
            })
        }
    })
}

fn convert_witness(witness: &RpcExecutionWitness, block_number: u64) -> Result<ExecutionWitness> {
    let headers = normalize_headers(&witness.headers, block_number)?;
    Ok(ExecutionWitness {
        state: ProgressiveList::from(byte_lists::<MAX_BYTES_PER_WITNESS_NODE>(
            &witness.state,
            "witness.state",
        )?),
        codes: ProgressiveList::from(byte_lists::<MAX_BYTES_PER_CODE>(
            &witness.codes,
            "witness.codes",
        )?),
        headers: SszList::<SszList<u8, MAX_BYTES_PER_HEADER>, MAX_WITNESS_HEADERS>::try_from(
            byte_lists::<MAX_BYTES_PER_HEADER>(&headers, "witness.headers")?,
        )
        .map_err(|err| anyhow::anyhow!("witness.headers exceeds the SSZ item bound: {err:?}"))?,
    })
}

fn byte_lists<const MAX: usize>(values: &[Bytes], label: &str) -> Result<Vec<SszList<u8, MAX>>> {
    values
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            SszList::<u8, MAX>::try_from(bytes.to_vec())
                .map_err(|err| anyhow::anyhow!("{label}[{i}] exceeds the SSZ byte bound: {err:?}"))
        })
        .collect()
}

/// Ancestor headers in ascending order, contiguous, ending with the parent.
fn normalize_headers(headers: &[Bytes], block_number: u64) -> Result<Vec<Bytes>> {
    ensure!(block_number > 0, "the genesis block has no parent header");
    let mut headers = headers
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let mut slice = bytes.as_ref();
            let header = Header::decode(&mut slice)
                .with_context(|| format!("witness header #{i} is not valid RLP"))?;
            ensure!(slice.is_empty(), "witness header #{i} has trailing bytes");
            Ok((header.number, bytes.clone()))
        })
        .collect::<Result<Vec<_>>>()?;
    headers.sort_by_key(|(number, _)| *number);
    for pair in headers.windows(2) {
        let (prev, next) = (pair[0].0, pair[1].0);
        ensure!(next != prev, "duplicate witness header for block #{next}");
        ensure!(
            next == prev + 1,
            "witness headers are not contiguous: #{prev} then #{next}"
        );
    }
    ensure!(
        headers
            .last()
            .is_some_and(|(number, _)| *number + 1 == block_number),
        "witness is missing the parent header of block #{block_number}"
    );
    Ok(headers.into_iter().map(|(_, bytes)| bytes).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sepolia_forks_by_timestamp() {
        // Block 11870794 (2026-10-09), after Glamsterdam on 2026-10-06.
        assert_eq!(
            fork_at(11155111, 0x6ac7aa14).unwrap(),
            ProtocolFork::Amsterdam
        );
        assert_eq!(
            fork_at(11155111, SEPOLIA_AMSTERDAM_TIMESTAMP - 1).unwrap(),
            ProtocolFork::BPO2
        );
        assert_eq!(
            fork_at(11155111, SEPOLIA_PRAGUE_TIMESTAMP).unwrap(),
            ProtocolFork::Prague
        );
        assert!(fork_at(11155111, SEPOLIA_PARIS_TIMESTAMP - 1).is_err());
    }

    #[test]
    fn mainnet_has_no_amsterdam_yet() {
        assert_eq!(
            fork_at(1, MAINNET_BPO2_TIMESTAMP).unwrap(),
            ProtocolFork::BPO2
        );
        assert_eq!(fork_at(1, u64::MAX).unwrap(), ProtocolFork::BPO2);
        assert_eq!(
            fork_at(1, MAINNET_CANCUN_TIMESTAMP).unwrap(),
            ProtocolFork::Cancun
        );
    }

    #[test]
    fn unknown_chain_is_rejected() {
        assert!(fork_at(31337, u64::MAX).is_err());
    }
}

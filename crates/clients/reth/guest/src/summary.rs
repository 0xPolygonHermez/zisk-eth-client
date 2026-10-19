//! Block-level facts decoded from a stateless input, for logging and
//! for the host's input statistics. Decoding the input twice (here and inside
//! the validator) is cheap relative to block execution.

use stateless_validator_common::{
    guest::{
        input::{new_payload_request::NewPayloadRequest, ProtocolFork},
        Error, StatelessInput, StatelessValidationResult,
    },
    HashTreeRoot as _, Sha2Hasher, SszEncode as _,
};

/// The output a successful validation of `input` must produce: the SSZ
/// `StatelessValidationResult` with the payload request root, the chain id
/// and the schema id of the input. Host-side helper (it decodes the input).
pub fn expected_output(input: &[u8]) -> Result<Vec<u8>, Error> {
    let (fork, input) = StatelessInput::from_schema_prefixed_ssz(input)?;
    Ok(StatelessValidationResult {
        new_payload_request_root: input.new_payload_request.hash_tree_root(&Sha2Hasher),
        successful_validation: true,
        chain_id: input.chain_id,
        schema_id: fork.schema_id(),
    }
    .to_ssz())
}

/// `(block_hash, gas_used, tx_count)` of any `NewPayloadRequest*` variant: the
/// execution payload structs share these fields but not a trait.
macro_rules! payload_facts {
    ($request:expr) => {{
        let payload = &$request.execution_payload;
        (
            payload.block_hash,
            payload.gas_used,
            payload.transactions.len(),
        )
    }};
}

#[derive(Debug, Clone, Copy)]
pub struct InputSummary {
    pub fork: ProtocolFork,
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub timestamp: u64,
    pub gas_used: u64,
    pub tx_count: usize,
}

impl InputSummary {
    /// Decode the summary from schema-prefixed SSZ input bytes.
    pub fn decode(input: &[u8]) -> Result<Self, Error> {
        let (fork, input) = StatelessInput::from_schema_prefixed_ssz(input)?;
        Ok(Self::from_input(fork, &input))
    }

    pub fn from_input(fork: ProtocolFork, input: &StatelessInput) -> Self {
        let request = &input.new_payload_request;
        let (block_hash, gas_used, tx_count) = match request {
            NewPayloadRequest::Bellatrix(r) => payload_facts!(r),
            NewPayloadRequest::Capella(r) => payload_facts!(r),
            NewPayloadRequest::Deneb(r) => payload_facts!(r),
            NewPayloadRequest::ElectraFulu(r) => payload_facts!(r),
            NewPayloadRequest::Gloas(r) => payload_facts!(r),
        };
        Self {
            fork,
            chain_id: input.chain_id,
            block_number: request.block_number(),
            block_hash,
            timestamp: request.timestamp(),
            gas_used,
            tx_count,
        }
    }
}

/// Block facts read straight from the stateless input bytes, without
/// decoding it: a handful of integer reads at fixed SSZ offsets. For logging
/// only; `None` if the bytes are too short to hold the fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPeek {
    pub chain_id: u64,
    pub block_number: u64,
    pub gas_used: u64,
    pub tx_count: usize,
}

impl InputPeek {
    /// SSZ layout used (all integers little-endian, offsets `u32`):
    /// - input = `schema_id (2 bytes BE) || StatelessInput`
    /// - `StatelessInput`: offset(new_payload_request) | offset(witness) | chain_id
    /// - `NewPayloadRequest*`: its first field is always the execution payload,
    ///   so its offset is the first `u32`
    /// - `ExecutionPayloadV*`: 404 bytes of fixed fields, then block_number,
    ///   gas_limit, gas_used, timestamp, offset(extra_data), base_fee (32),
    ///   block_hash (32), offset(transactions)
    /// - `transactions`: a list of variable-size items, so its first `u32` is
    ///   the offset of the first item, which is 4 times the item count.
    pub fn peek(input: &[u8]) -> Option<Self> {
        const SCHEMA: usize = 2;
        const BLOCK_NUMBER: usize = 404;
        const GAS_USED: usize = 420;
        const EXTRA_DATA_OFFSET: usize = 436;
        const TRANSACTIONS_OFFSET: usize = 504;

        let u32_at = |at: usize| -> Option<usize> {
            input
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
        };
        let u64_at = |at: usize| -> Option<u64> {
            input
                .get(at..at + 8)
                .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
        };

        let chain_id = u64_at(SCHEMA + 8)?;
        let request = SCHEMA + u32_at(SCHEMA)?;
        let payload = request + u32_at(request)?;
        let block_number = u64_at(payload + BLOCK_NUMBER)?;
        let gas_used = u64_at(payload + GAS_USED)?;
        let tx_start = payload + u32_at(payload + TRANSACTIONS_OFFSET)?;
        // The list ends where the next variable field starts: `withdrawals`
        // (offset right after the transactions offset) from V2 on, else the
        // end of the request, which is where the witness starts. V1 is told
        // apart by its fixed part ending right after the transactions offset,
        // which is where its first variable field (`extra_data`) begins.
        let extra_data_off = u32_at(payload + EXTRA_DATA_OFFSET)?;
        let tx_end = if extra_data_off > TRANSACTIONS_OFFSET + 4 {
            payload + u32_at(payload + TRANSACTIONS_OFFSET + 4)?
        } else {
            SCHEMA + u32_at(SCHEMA + 4)?
        };
        // An empty list has no offset table; a non-empty one starts with the
        // offset of its first item, which is 4 times the item count.
        let tx_count = if tx_end >= tx_start + 4 {
            u32_at(tx_start)? / 4
        } else {
            0
        };

        Some(Self {
            chain_id,
            block_number,
            gas_used,
            tx_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoke_input() -> Vec<u8> {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../testdata/guest-smoke.json")).unwrap();
        let hex = json["statelessInputBytes"].as_str().unwrap();
        (2..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn peek_matches_full_decode() {
        let input = smoke_input();
        let full = InputSummary::decode(&input).unwrap();
        let peek = InputPeek::peek(&input).unwrap();
        assert_eq!(peek.chain_id, full.chain_id);
        assert_eq!(peek.block_number, full.block_number);
        assert_eq!(peek.gas_used, full.gas_used);
        assert_eq!(peek.tx_count, full.tx_count);
        assert_eq!(full.block_number, 1);
    }

    #[test]
    fn expected_output_matches_fixture() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../testdata/guest-smoke.json")).unwrap();
        let hex = json["statelessOutputBytes"].as_str().unwrap();
        let expected: Vec<u8> = (2..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(expected_output(&smoke_input()).unwrap(), expected);
    }

    #[test]
    fn peek_rejects_short_input() {
        assert!(InputPeek::peek(&[0x15, 0x01, 0, 0]).is_none());
    }
}

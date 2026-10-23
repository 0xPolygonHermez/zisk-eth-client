//! Cursor over a ZEG0 container — the inverse of `rust-input-gen`'s `Writer`.
//!
//! Layout is defined by `rust-input-gen`'s `sections.rs` / `state_root.rs`;
//! `third_party/ziskethone/BINARY_FORMAT.md` lags behind them.

use alloy_primitives::{Address, Bytes, B256, U256};
use anyhow::{bail, ensure, Result};

/// Format versions this reader understands, mirroring `sections::FORMAT_VERSION`.
///
/// v9: compact StateRoot — branches tag their 16 children in one word, nibble
///     runs pack two per byte (see `trie.rs`).
/// v10: Amsterdam fields — ConsensusInfo prefix 368 → 408 B, PreviousBlocks
///      records 728 → 768 B (block_access_list_hash + slot_number).
/// v11: ConsensusInfo prefix 408 → 376 B (the BAL hash leaves it), plus a
///      trailing BalHint section after StateRoot, which the trie walk ignores.
/// v12: sorted-order links — the StateRoot header gains a fourth u64, leaves
///      carry link words, and absent accessed keys appear as empty leaves.
/// v13: a 16-byte hash seed right after the magic + version.
pub const SUPPORTED_VERSIONS: &[u32] = &[8, 9, 10, 11, 12, 13];

/// Byte length of the ConsensusInfo fixed prefix.
fn consensus_prefix_len(version: u32) -> usize {
    match version {
        ..=9 => 368,
        10 => 408,
        _ => 376,
    }
}

/// One EIP-4895 withdrawal record.
const WITHDRAWAL_RECORD_LEN: usize = 48;

/// One PreviousBlocks record.
fn prev_block_record_len(version: u32) -> usize {
    if version >= 10 {
        768
    } else {
        728
    }
}

pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        ensure!(
            self.pos + n <= self.buf.len(),
            "unexpected end of ZEG0 stream: wanted {n} B at offset {}, only {} B left",
            self.pos,
            self.remaining()
        );
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn u32_le(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn u64_le(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn b256(&mut self) -> Result<B256> {
        Ok(B256::from_slice(self.take(32)?))
    }

    pub fn u256_be(&mut self) -> Result<U256> {
        Ok(U256::from_be_slice(self.take(32)?))
    }

    pub fn address(&mut self) -> Result<Address> {
        Ok(Address::from_slice(self.take(20)?))
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }

    /// Advance to the next 8-byte boundary (no-op when already aligned).
    pub fn align8(&mut self) -> Result<()> {
        let rem = self.pos % 8;
        if rem != 0 {
            self.skip(8 - rem)?;
        }
        Ok(())
    }

    /// `u64` length prefix, then that many bytes, then pad to 8.
    pub fn len_prefixed(&mut self) -> Result<Vec<u8>> {
        let len = self.u64_le()? as usize;
        let v = self.take(len)?.to_vec();
        self.align8()?;
        Ok(v)
    }
}

/// A withdrawal, as carried in ConsensusInfo.
#[derive(Debug, Clone)]
pub struct Withdrawal {
    pub index: u64,
    pub validator_index: u64,
    pub address: Address,
    pub amount: u64,
}

/// The header fields ZEG0 does carry for the current block. The execution
/// outputs it omits (state_root, receipts_root, logs_bloom, gas_used,
/// blob_gas_used) come from RPC, or from re-execution with `--offline`.
#[derive(Debug, Clone)]
pub struct ConsensusInfo {
    /// The parent's **state root** — this slot is not a parent block hash.
    pub parent_state_root: B256,
    pub beneficiary: Address,
    pub number: u64,
    pub gas_limit: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: B256,
    pub parent_beacon_block_root: B256,
    pub base_fee_per_gas: U256,
    pub excess_blob_gas: u64,
    pub requests_hash: B256,
    pub difficulty: U256,
    pub nonce: [u8; 8],
    pub ommers_hash: B256,
    /// The guest's `ForkId`: 1 Berlin, 2 London, 4 Shanghai, 5 Cancun,
    /// 6 Prague, 7 Osaka, 8 Amsterdam. Decides which optional header fields
    /// the block has.
    pub fork_id: u64,
}

/// A fully-specified ancestor header, enough to re-encode its RLP.
#[derive(Debug, Clone)]
pub struct PrevBlock {
    pub parent_hash: B256,
    pub ommers_hash: B256,
    pub beneficiary: Address,
    /// How many header fields to RLP-encode (fork-dependent: 15..=23).
    pub field_count: u32,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: Vec<u8>,
    pub difficulty: U256,
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: B256,
    pub nonce: [u8; 8],
    pub base_fee_per_gas: U256,
    pub withdrawals_root: B256,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
    pub parent_beacon_block_root: B256,
    pub requests_hash: B256,
    /// Zero before v10 and pre-Amsterdam.
    pub block_access_list_hash: B256,
    pub slot_number: u64,
}

/// Everything the transcoder needs out of a ZEG0 file.
pub struct Zeg0 {
    /// The container's format version (one of `SUPPORTED_VERSIONS`). The trie
    /// walk needs it: v9 packs nibbles and tags branch children, v12 adds link
    /// words to leaves.
    pub version: u32,
    pub consensus: ConsensusInfo,
    pub withdrawals: Vec<Withdrawal>,
    /// Canonical EIP-2718 wire envelopes, in block order.
    pub transactions: Vec<Bytes>,
    /// Deployed bytecode. A superset of the original `witness.codes`: the
    /// encoder also synthesizes EIP-7702 delegation stubs. Harmless for reth,
    /// which indexes code by hash.
    pub codes: Vec<Bytes>,
    /// Index 0 is the parent.
    pub prev_blocks: Vec<PrevBlock>,
    /// The raw trie-hint opcode stream (header counts already consumed).
    pub trie_stream: Vec<u8>,
}

/// A committed ziskethone input is a `ZiskStdin` container holding the ZEG0
/// bytes in its first frame. Also accepts a bare ZEG0 payload.
fn unwrap_zisk_frame(buf: &[u8]) -> Result<&[u8]> {
    if buf.starts_with(b"ZEG0") {
        return Ok(buf);
    }
    input_core::first_frame(buf)
}

pub fn parse(outer: &[u8]) -> Result<Zeg0> {
    let buf = unwrap_zisk_frame(outer)?;
    let mut c = Cursor::new(buf);

    // Section 0 — magic.
    let magic = c.take(4)?;
    if magic != b"ZEG0" {
        bail!("not a ZEG0 file (magic was {magic:02x?})");
    }
    let version = c.u32_le()?;
    ensure!(
        SUPPORTED_VERSIONS.contains(&version),
        "unsupported ZEG0 version {version} (this tool understands {SUPPORTED_VERSIONS:?})"
    );
    if version >= 13 {
        c.skip(16)?; // hash seed: guest-internal, never affects the block
    }

    // Section 1 — ConsensusInfo.
    let prefix_start = c.pos;
    let parent_state_root = c.b256()?;
    let beneficiary = c.address()?;
    c.skip(4)?; // pad
    let number = c.u64_le()?;
    let gas_limit = c.u64_le()?;
    let timestamp = c.u64_le()?;
    let extra_len = c.u64_le()? as usize;
    ensure!(extra_len <= 32, "extra_data_len {extra_len} exceeds 32");
    let extra_data = c.take(32)?[..extra_len].to_vec();
    let mix_hash = c.b256()?;
    let parent_beacon_block_root = c.b256()?;
    let base_fee_per_gas = c.u256_be()?;
    let withdrawals_count = c.u64_le()? as usize;
    let excess_blob_gas = c.u64_le()?;
    let requests_hash = c.b256()?;
    let difficulty = c.u256_be()?;
    let nonce: [u8; 8] = c.take(8)?.try_into().unwrap();
    let ommers_hash = c.b256()?;
    let fork_id = c.u64_le()?;
    // Skip to the end of the fixed prefix — the blob schedule (and v10+'s
    // Amsterdam fields) are re-derived by the encoder, not read back.
    let consumed = c.pos - prefix_start;
    c.skip(consensus_prefix_len(version) - consumed)?;

    let mut withdrawals = Vec::with_capacity(withdrawals_count);
    for _ in 0..withdrawals_count {
        let start = c.pos;
        let index = c.u64_le()?;
        let validator_index = c.u64_le()?;
        let address = c.address()?;
        c.skip(4)?;
        let amount = c.u64_le()?;
        debug_assert_eq!(c.pos - start, WITHDRAWAL_RECORD_LEN);
        withdrawals.push(Withdrawal {
            index,
            validator_index,
            address,
            amount,
        });
    }

    // Section 2 — Transactions.
    let tx_count = c.u64_le()? as usize;
    let mut transactions = Vec::with_capacity(tx_count);
    for _ in 0..tx_count {
        transactions.push(Bytes::from(c.len_prefixed()?));
    }

    // Section 4 — Contracts.
    let code_count = c.u64_le()? as usize;
    let mut codes = Vec::with_capacity(code_count);
    for _ in 0..code_count {
        codes.push(Bytes::from(c.len_prefixed()?));
    }

    // Section 6 — PreviousBlocks.
    let prev_count = c.u64_le()? as usize;
    let mut prev_blocks = Vec::with_capacity(prev_count);
    for _ in 0..prev_count {
        let start = c.pos;
        let parent_hash = c.b256()?;
        let ommers_hash = c.b256()?;
        let beneficiary = c.address()?;
        // 84..88 — `field_count` sits directly after the 20-byte coinbase. (The
        // format doc lists a pad here; the encoder writes field_count instead.)
        let field_count = c.u32_le()?;
        let state_root = c.b256()?;
        let transactions_root = c.b256()?;
        let receipts_root = c.b256()?;
        let logs_bloom = c.take(256)?.to_vec();
        let difficulty = c.u256_be()?;
        let number = c.u64_le()?;
        let gas_limit = c.u64_le()?;
        let gas_used = c.u64_le()?;
        let timestamp = c.u64_le()?;
        let ed_len = c.u64_le()? as usize;
        ensure!(ed_len <= 32, "ancestor extra_data_len {ed_len} exceeds 32");
        let extra_buf = c.take(32)?;
        let extra_data = extra_buf[..ed_len].to_vec();
        let mix_hash = c.b256()?;
        let nonce: [u8; 8] = c.take(8)?.try_into().unwrap();
        let base_fee_per_gas = c.u256_be()?;
        let withdrawals_root = c.b256()?;
        let blob_gas_used = c.u64_le()?;
        let excess_blob_gas = c.u64_le()?;
        let parent_beacon_block_root = c.b256()?;
        let requests_hash = c.b256()?;
        let (block_access_list_hash, slot_number) = if version >= 10 {
            (c.b256()?, c.u64_le()?)
        } else {
            (B256::ZERO, 0)
        };
        ensure!(
            c.pos - start == prev_block_record_len(version),
            "PreviousBlocks record was {} B, expected {}",
            c.pos - start,
            prev_block_record_len(version)
        );
        prev_blocks.push(PrevBlock {
            parent_hash,
            ommers_hash,
            beneficiary,
            field_count,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            nonce,
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
            block_access_list_hash,
            slot_number,
        });
    }

    // Section 7 — StateRoot trie hints. Three u64 counts (v12+: plus the head
    // of the sorted-account links), then the stream, which runs to end of file.
    let _number_of_nodes = c.u64_le()?;
    let _number_of_accounts = c.u64_le()?;
    let _number_of_storages = c.u64_le()?;
    if version >= 12 {
        let _first_sorted_account = c.u64_le()?;
    }
    let trie_stream = c.take(c.remaining())?.to_vec();

    Ok(Zeg0 {
        version,
        consensus: ConsensusInfo {
            parent_state_root,
            beneficiary,
            number,
            gas_limit,
            timestamp,
            extra_data,
            mix_hash,
            parent_beacon_block_root,
            base_fee_per_gas,
            excess_blob_gas,
            requests_hash,
            difficulty,
            nonce,
            ommers_hash,
            fork_id,
        },
        withdrawals,
        transactions,
        codes,
        prev_blocks,
        trie_stream,
    })
}

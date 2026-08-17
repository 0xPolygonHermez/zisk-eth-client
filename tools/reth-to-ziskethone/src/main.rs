//! Offline transcoder: reth stateless-validator `.bin` → ziskethone `ZEG0`
//! input `.bin`, without any RPC.
//!
//! The reth and ziskethone inputs describe the *same* block execution: a block
//! plus its `debug_executionWitness` (state trie nodes, codes, keys, ancestor
//! headers). For a given block both files were produced from identical raw
//! material — so a committed reth `.bin` already contains everything the ZEG0
//! encoder needs. This lets us regenerate ziskethone inputs for blocks that are
//! now too old for a normal RPC to serve state.
//!
//! Pipeline (mirrors `rust_input_gen::live::fetch_offline_sources_online`, but
//! sourced from the reth file instead of the network):
//!   1. Read the reth `.bin`'s two ZiskStdin slices (public, witness).
//!   2. From the public slice, bincode-decode the leading `Vec<u8>` (the
//!      RLP-encoded block — the `BlockRlp` serde adapter in guest-reth writes
//!      the block as its first field) and RLP-decode it into an alloy
//!      consensus block. The trailing chain_config/public_keys are ignored.
//!   3. From the witness slice, bincode-decode the `ExecutionWitness`
//!      (field-identical between `alloy_rpc_types_debug` and rust-input-gen).
//!   4. Derive parent + ancestors from `witness.headers`, fork/blob scalars
//!      from the block timestamp (compiled-in mainnet schedule).
//!   5. `encode_binary` → ZEG0 bytes → wrap in one length-prefixed ZiskStdin
//!      slice, matching what the ziskethone client writes.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use serde::Deserialize;

use alloy::consensus::transaction::Recovered;
use alloy::consensus::{Block as ConsensusBlock, Header as ConsensusHeader, TxEnvelope};
use alloy::primitives::{Address, Bytes, B256};
use alloy::rpc::types::{
    Block as RpcBlock, BlockTransactions, Header as RpcHeader, Transaction as RpcTx,
};
use alloy_rlp::Decodable;

use rust_input_gen::offline::{encode_binary, OfflineSources};
use rust_input_gen::rpc::{ExecutionWitness, Prestate, PrestateDiff};

#[derive(Parser, Debug)]
#[command(version, about = "Transcode reth stateless-validator .bin files into ziskethone ZEG0 inputs (offline, no RPC)")]
struct Cli {
    /// A reth `.bin` file, or a directory of them.
    input: PathBuf,

    /// Output directory for the generated `*_zec_ziskethone.bin` files.
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Chain slug used in the output filename (these inputs are mainnet).
    #[arg(long, default_value = "mainnet")]
    chain: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("creating output dir {}", cli.output_dir.display()))?;

    let inputs: Vec<PathBuf> = if cli.input.is_dir() {
        let mut v: Vec<_> = fs::read_dir(&cli.input)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        v.sort();
        v
    } else {
        vec![cli.input.clone()]
    };

    let mut failures = 0usize;
    for in_path in &inputs {
        match convert_file(in_path, &cli.output_dir, &cli.chain) {
            Ok(out) => println!("ok  {} -> {}", in_path.display(), out.display()),
            Err(e) => {
                failures += 1;
                eprintln!("err {}: {e:#}", in_path.display());
            }
        }
    }
    if failures > 0 {
        bail!("{failures}/{} file(s) failed", inputs.len());
    }
    Ok(())
}

fn convert_file(in_path: &Path, out_dir: &Path, chain: &str) -> Result<PathBuf> {
    let bytes = fs::read(in_path).with_context(|| format!("read {}", in_path.display()))?;

    // Two ZiskStdin slices: [public][witness].
    let mut cursor = 0usize;
    let public_slice = read_zisk_slice(&bytes, &mut cursor).context("read public slice")?;
    let witness_slice = read_zisk_slice(&bytes, &mut cursor).context("read witness slice")?;

    let block = decode_block(public_slice).context("decode block from public slice")?;
    let witness = decode_witness(witness_slice).context("decode witness slice")?;

    let sources = build_sources(block, witness).context("assemble OfflineSources")?;
    let stats = &sources.0;
    let zeg0 = encode_binary(&sources.1).context("encode ZEG0 container")?;

    let filename = format!(
        "{}_{}_{}_{}_zec_ziskethone.bin",
        chain.to_lowercase(),
        stats.number,
        stats.tx_count,
        stats.gas_used / 1_000_000,
    );
    let out_path = out_dir.join(filename);

    let mut out = Vec::with_capacity(zeg0.len() + 8);
    write_zisk_slice(&mut out, &zeg0);
    fs::write(&out_path, &out).with_context(|| format!("write {}", out_path.display()))?;
    Ok(out_path)
}

/// The stats needed only for the output filename.
struct Stats {
    number: u64,
    tx_count: usize,
    gas_used: u64,
}

/// Decode the RLP-encoded block that leads the reth public slice. The public
/// slice is `bincode(RethInputPublic { block: Vec<u8>(rlp), chain_config, .. })`;
/// bincode lays fields out in order, so the leading value is the RLP `Vec<u8>`.
fn decode_block(public_slice: &[u8]) -> Result<ConsensusBlock<TxEnvelope>> {
    let cfg = bincode::config::standard();
    let (rlp_bytes, _): (Vec<u8>, usize) =
        bincode::serde::decode_from_slice(public_slice, cfg).context("bincode Vec<u8>(rlp block)")?;
    ConsensusBlock::<TxEnvelope>::decode(&mut rlp_bytes.as_slice())
        .map_err(|e| anyhow!("RLP-decode block: {e:?}"))
}

/// The witness slice is `bincode(RethInputWitness { witness: ExecutionWitness })`.
/// We mirror that shape with a local struct so we don't depend on reth's crate.
#[derive(Deserialize)]
struct WitnessSlice {
    witness: RawWitness,
}
#[derive(Deserialize)]
struct RawWitness {
    state: Vec<Bytes>,
    codes: Vec<Bytes>,
    keys: Vec<Bytes>,
    headers: Vec<Bytes>,
}

fn decode_witness(witness_slice: &[u8]) -> Result<ExecutionWitness> {
    let cfg = bincode::config::standard();
    let (ws, _): (WitnessSlice, usize) =
        bincode::serde::decode_from_slice(witness_slice, cfg).context("bincode witness")?;
    Ok(ExecutionWitness {
        state: ws.witness.state,
        codes: ws.witness.codes,
        keys: ws.witness.keys,
        headers: ws.witness.headers,
    })
}

fn build_sources(
    block: ConsensusBlock<TxEnvelope>,
    witness: ExecutionWitness,
) -> Result<(Stats, OfflineSources)> {
    let stats = Stats {
        number: block.header.number,
        tx_count: block.body.transactions.len(),
        gas_used: block.header.gas_used,
    };
    let parent_hash = block.header.parent_hash;
    let timestamp = block.header.timestamp;

    // Consensus block -> rpc block with Full transactions. The ZEG0 encoder
    // reads header fields, tx envelopes (via `tx.inner`), and withdrawals; the
    // recovered sender is never read, so Address::ZERO is fine.
    let header = RpcHeader::new(block.header.clone());
    let txs: Vec<RpcTx> = block
        .body
        .transactions
        .into_iter()
        .map(|env| RpcTx {
            inner: Recovered::new_unchecked(env, Address::ZERO),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
            block_timestamp: None,
        })
        .collect();
    let current = RpcBlock {
        header,
        uncles: vec![],
        transactions: BlockTransactions::Full(txs),
        withdrawals: block.body.withdrawals,
    };

    let parent = parent_from_witness(&witness, parent_hash)?;
    let ancestors = build_ancestors_from_witness(&witness, &parent);
    let (is_osaka, blob_base_fee_update_fraction, target_blob_gas_per_block, max_blob_gas_per_block) =
        mainnet_fork_params(timestamp);

    let sources = OfflineSources {
        current,
        parent,
        ancestors,
        prestate: Prestate::default(),
        diff: PrestateDiff::default(),
        witness,
        system_contract_slots: BTreeSet::new(),
        is_osaka,
        blob_base_fee_update_fraction,
        target_blob_gas_per_block,
        max_blob_gas_per_block,
    };
    Ok((stats, sources))
}

// ---------------------------------------------------------------------------
// Helpers copied verbatim (behavior-for-behavior) from rust-input-gen's
// `fetch.rs` (private there). Keep in sync if the submodule's schedule/logic
// changes. `mainnet_fork_params` in particular MUST be updated at each mainnet
// fork / blob-schedule (BPO) change.
// ---------------------------------------------------------------------------

fn mainnet_fork_params(timestamp: u64) -> (bool, u64, u64, u64) {
    const OSAKA_ACTIVATION: u64 = 1767747671;
    const GAS_PER_BLOB: u64 = 131_072;
    // (activation_time, base_fee_update_fraction, target_blob_count, max_blob_count), newest first.
    const SCHEDULE: &[(u64, u64, u64, u64)] = &[
        (1767747671, 11684671, 14, 21), // BPO2
        (1765290071, 8346193, 10, 15),  // BPO1
        (1746612311, 5007716, 6, 9),    // Prague
        (1710338135, 3338477, 3, 6),    // Cancun
    ];
    let is_osaka = timestamp >= OSAKA_ACTIVATION;
    let (fraction, target_blobs, max_blobs) = SCHEDULE
        .iter()
        .find(|(act, _, _, _)| timestamp >= *act)
        .map(|(_, f, t, m)| (*f, *t, *m))
        .unwrap_or((0, 0, 0));
    (
        is_osaka,
        fraction,
        target_blobs * GAS_PER_BLOB,
        max_blobs * GAS_PER_BLOB,
    )
}

fn parent_from_witness(witness: &ExecutionWitness, parent_hash: B256) -> Result<RpcBlock> {
    for raw in &witness.headers {
        let mut slice: &[u8] = raw.as_ref();
        if let Ok(h) = ConsensusHeader::decode(&mut slice) {
            if h.hash_slow() == parent_hash {
                return Ok(RpcBlock {
                    header: RpcHeader::new(h),
                    ..Default::default()
                });
            }
        }
    }
    bail!(
        "parent header {parent_hash} not in witness.headers ({} headers present)",
        witness.headers.len()
    );
}

fn build_ancestors_from_witness(witness: &ExecutionWitness, parent: &RpcBlock) -> Vec<RpcBlock> {
    let parent_number = parent.header.number;

    let mut headers: Vec<ConsensusHeader> = Vec::with_capacity(witness.headers.len());
    for raw in &witness.headers {
        let mut slice: &[u8] = raw.as_ref();
        if let Ok(h) = ConsensusHeader::decode(&mut slice) {
            headers.push(h);
        }
    }

    headers.sort_by_key(|h| std::cmp::Reverse(h.number));
    headers.dedup_by_key(|h| h.number);

    let mut out: Vec<RpcBlock> = Vec::with_capacity(headers.len() + 1);
    out.push(parent.clone());
    for h in headers.into_iter() {
        if h.number == parent_number {
            continue;
        }
        out.push(RpcBlock {
            header: RpcHeader::new(h),
            ..Default::default()
        });
    }
    out
}

// ---------------------------------------------------------------------------
// ZiskStdin slice framing: u64-le length prefix + payload + pad to 8 bytes.
// Copied from tools/migrate-inputs (which reads the same reth `.bin` format).
// ---------------------------------------------------------------------------

fn read_zisk_slice<'a>(buf: &'a [u8], cursor: &mut usize) -> Result<&'a [u8]> {
    if *cursor + 8 > buf.len() {
        return Err(anyhow!("truncated length prefix at offset {cursor}"));
    }
    let len = u64::from_le_bytes(buf[*cursor..*cursor + 8].try_into().unwrap()) as usize;
    *cursor += 8;
    if *cursor + len > buf.len() {
        return Err(anyhow!("slice len {len} extends past buffer"));
    }
    let slice = &buf[*cursor..*cursor + len];
    *cursor += len;
    let padding = (8 - ((8 + len) % 8)) % 8;
    *cursor += padding;
    Ok(slice)
}

fn write_zisk_slice(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(data);
    let padding = (8 - ((8 + data.len()) % 8)) % 8;
    out.extend(std::iter::repeat(0u8).take(padding));
}

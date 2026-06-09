//! One-shot migrator: rewrites pre-v2.1.0 RethInput `.bin` files into the
//! new format used after the reth v1.11.0 → v2.1.0 upgrade.
//!
//! Old slice 1 (bincode):  block_bincode_compat || chain_config_v1 || public_keys
//! New slice 1 (bincode):  bincode(Vec<u8>(rlp(block))) || chain_config_v2 || public_keys
//!
//! Two structural changes between versions:
//!   1. Block field switched from `serde_bincode_compat::Block` to RLP-wrapped `Vec<u8>`.
//!   2. `alloy_genesis::serde_bincode_compat::ChainConfig` inserted
//!      `amsterdam_time: Option<u64>` at position 21 (between `osaka_time` and `bpo1_time`).
//!
//! Slice 2 (RethInputWitness / ExecutionWitness) and the public_keys vec are
//! bincode-identical between versions, so we pass them through.
//!
//! Delete this directory once migration is complete.

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use alloy_consensus::Header;
use alloy_rlp::Encodable;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reth_ethereum_primitives::{Block, TransactionSigned};
use serde::Deserialize;
use serde_with::serde_as;

#[derive(Parser, Debug)]
#[command(version, about = "Migrate legacy RethInput .bin files to the post-v2.1.0 format")]
struct Cli {
    /// File or directory of legacy `.bin` files to migrate.
    input: PathBuf,

    /// Output directory. Defaults to writing alongside each input with
    /// `.bin` replaced by `.new.bin`.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
}

/// Decodes just the leading `Block` of an old slice-1.
#[serde_as]
#[derive(Debug, Deserialize)]
struct BlockOnly {
    #[serde_as(
        as = "reth_primitives_traits::serde_bincode_compat::Block<TransactionSigned, Header>"
    )]
    block: Block,
}

/// The leading 21 fields of `alloy_genesis::serde_bincode_compat::ChainConfig`
/// (v1.7.3 layout: chain_id through osaka_time). Same wire layout in v2.0.0 up
/// to and including this struct — v2.0.0 inserts `amsterdam_time` AFTER these
/// 21 fields, which we inject manually as `Option::None` (0x00).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChainConfigPrefix {
    chain_id: u64,
    homestead_block: Option<u64>,
    dao_fork_block: Option<u64>,
    dao_fork_support: bool,
    eip150_block: Option<u64>,
    eip155_block: Option<u64>,
    eip158_block: Option<u64>,
    byzantium_block: Option<u64>,
    constantinople_block: Option<u64>,
    petersburg_block: Option<u64>,
    istanbul_block: Option<u64>,
    muir_glacier_block: Option<u64>,
    berlin_block: Option<u64>,
    london_block: Option<u64>,
    arrow_glacier_block: Option<u64>,
    gray_glacier_block: Option<u64>,
    merge_netsplit_block: Option<u64>,
    shanghai_time: Option<u64>,
    cancun_time: Option<u64>,
    prague_time: Option<u64>,
    osaka_time: Option<u64>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let inputs: Vec<PathBuf> = if cli.input.is_dir() {
        let mut v: Vec<_> = fs::read_dir(&cli.input)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            // Skip prior outputs: `foo.new.bin` has extension "bin" but stem ends in ".new".
            .filter(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| !s.ends_with(".new"))
                    .unwrap_or(true)
            })
            .collect();
        v.sort();
        v
    } else {
        vec![cli.input.clone()]
    };

    if let Some(dir) = &cli.output_dir {
        fs::create_dir_all(dir)?;
    }

    for in_path in &inputs {
        let out_path = match &cli.output_dir {
            Some(dir) => dir.join(in_path.file_name().unwrap()),
            None => in_path.with_extension("new.bin"),
        };
        match migrate_file(in_path, &out_path) {
            Ok(()) => println!("ok  {} -> {}", in_path.display(), out_path.display()),
            Err(e) => eprintln!("err {}: {e:#}", in_path.display()),
        }
    }

    Ok(())
}

fn migrate_file(in_path: &Path, out_path: &Path) -> Result<()> {
    let bytes = fs::read(in_path).with_context(|| format!("read {}", in_path.display()))?;

    let mut cursor = 0usize;
    let public_slice = read_zisk_slice(&bytes, &mut cursor).context("read public slice")?;
    let witness_slice = read_zisk_slice(&bytes, &mut cursor).context("read witness slice")?;

    let new_public = rewrite_public(public_slice).context("rewrite public slice")?;

    // Witness slice is wire-identical between versions.
    let mut out = Vec::with_capacity(new_public.len() + witness_slice.len() + 32);
    write_zisk_slice(&mut out, &new_public);
    write_zisk_slice(&mut out, witness_slice);

    fs::write(out_path, &out).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

/// Transform the bincode payload of slice 1:
///   - replace the leading bincode_compat Block with `bincode(Vec<u8>(rlp(block)))`
///   - inject `Option::None` (0x00) for `amsterdam_time` after the first 21
///     ChainConfig fields (osaka_time)
///   - leave everything else byte-identical
fn rewrite_public(old_slice: &[u8]) -> Result<Vec<u8>> {
    let cfg = bincode::config::standard();
    let mut reader = Cursor::new(old_slice);

    let BlockOnly { block } = bincode::serde::decode_from_std_read(&mut reader, cfg)
        .context("decode legacy bincode_compat Block")?;
    let after_block = reader.position() as usize;

    let _: ChainConfigPrefix = bincode::serde::decode_from_std_read(&mut reader, cfg)
        .context("decode legacy ChainConfig prefix (chain_id..osaka_time)")?;
    let after_cc_prefix = reader.position() as usize;

    let cc_prefix_bytes = &old_slice[after_block..after_cc_prefix];
    let tail_bytes = &old_slice[after_cc_prefix..];

    // RLP-encode the block, then bincode-encode it as Vec<u8>. This matches the
    // `BlockRlp` serde adapter in crates/guest-reth/src/lib.rs.
    let mut rlp = Vec::with_capacity(block.length());
    block.encode(&mut rlp);
    let new_block_field = bincode::serde::encode_to_vec(&rlp, cfg)
        .context("bincode-encode RLP bytes as Vec<u8>")?;

    let mut out = Vec::with_capacity(
        new_block_field.len() + cc_prefix_bytes.len() + 1 + tail_bytes.len(),
    );
    out.extend_from_slice(&new_block_field);
    out.extend_from_slice(cc_prefix_bytes);
    out.push(0x00); // amsterdam_time = None
    out.extend_from_slice(tail_bytes);
    Ok(out)
}

/// Read one length-prefixed, 8-byte-aligned slice from a ZiskStdin buffer.
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
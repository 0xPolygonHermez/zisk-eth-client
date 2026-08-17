//! ziskethone ZEG0 `.bin` → reth stateless-validator `.bin`.
//!
//! The ZEG0 container carries the expensive half of a reth input — the pre-state
//! witness — but not the current block's execution outputs, which the guest
//! recomputes. Those come from one `eth_getBlockByNumber`, served by any full
//! node. That is the point: `input-gen rpc` needs `debug_executionWitness`,
//! which requires an archive node, so it fails for exactly the old blocks this
//! tool handles.
//!
//! See `docs/superpowers/specs/2026-07-30-ziskethone-to-reth-design.md`.

mod assemble;
mod zeg0;

use std::fs;
use std::path::{Path, PathBuf};

use alloy_consensus::Header;
use alloy_provider::{Provider, ProviderBuilder};
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use input_reth::RethClient;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Transcode ziskethone ZEG0 inputs into reth stateless-validator inputs"
)]
struct Cli {
    /// A ziskethone `.bin` file, or a directory of them.
    input: PathBuf,

    /// RPC endpoint. Only `eth_getBlockByNumber` is called, so a normal full
    /// node is enough — no archive state required. Not needed with --check-only.
    #[arg(short = 'u', long, required_unless_present = "check_only")]
    rpc_url: Option<String>,

    /// Output directory for the generated `*_zec_reth.bin` files.
    #[arg(short, long, required_unless_present = "check_only")]
    output_dir: Option<PathBuf>,

    /// Chain slug used in the output filename.
    #[arg(long, default_value = "mainnet")]
    chain: String,

    /// Decode and rebuild the trie, verify the root, then stop. Makes no RPC
    /// call and writes nothing — useful to check an input is convertible.
    #[arg(long, default_value_t = false)]
    check_only: bool,

    /// Also diff the rebuilt witness against a real reth `.bin` for the same
    /// block (a file, or a directory searched by block number). Offline. Every
    /// rebuilt trie node must appear in the reference witness.
    #[arg(long)]
    compare_reth: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cli = Cli::parse();

    let inputs = collect_inputs(&cli.input)?;
    if inputs.is_empty() {
        bail!("no .bin files found at {}", cli.input.display());
    }

    // `--check-only` is the same pipeline with the emit stage switched off, not
    // a separate path: decode + rebuild + verify runs identically either way,
    // and that is everything which can go wrong in the reconstruction. The RPC
    // leg only supplies header fields the container cannot carry.
    let emit = match (cli.rpc_url, cli.output_dir) {
        _ if cli.check_only => None,
        (Some(rpc_url), Some(output_dir)) => {
            fs::create_dir_all(&output_dir)
                .with_context(|| format!("creating output dir {}", output_dir.display()))?;
            let provider = ProviderBuilder::new()
                .connect(&rpc_url)
                .await
                .with_context(|| format!("connecting to RPC at {rpc_url}"))?;
            Some((provider, output_dir))
        }
        // Unreachable: clap enforces both unless --check-only.
        _ => bail!("--rpc-url and --output-dir are required without --check-only"),
    };

    let mut failures = 0usize;
    for path in &inputs {
        let result = process(path, cli.compare_reth.as_deref(), emit.as_ref(), &cli.chain).await;
        if let Err(e) = result {
            failures += 1;
            warn!("err {}: {e:#}", path.display());
        }
    }

    if failures > 0 {
        bail!("{failures}/{} file(s) failed", inputs.len());
    }
    Ok(())
}

fn collect_inputs(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_dir() {
        let mut v: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        v.sort();
        Ok(v)
    } else {
        Ok(vec![path.to_path_buf()])
    }
}

/// Read the witness out of a reth stateless-validator `.bin`. The container is
/// two `ZiskStdin` slices — `[public][witness]` — and the second is
/// `bincode(RethInputWitness)`.
fn reth_witness(path: &Path) -> Result<stateless_reth::ExecutionWitness> {
    let buf = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let slices = zeg0::reader::zisk_slices(&buf)?;
    let witness_bytes = slices.get(1).with_context(|| {
        format!(
            "{} has {} slice(s), expected 2",
            path.display(),
            slices.len()
        )
    })?;
    Ok(input_reth::guest::RethInputWitness::deserialize(witness_bytes)?.witness)
}

/// Every node we rebuilt must be present in the reference witness. The converse
/// need not hold: `debug_executionWitness` can reveal nodes the guest's trie
/// walk never reaches.
fn compare(rebuilt: &zeg0::trie::Rebuilt, reference_path: &Path, block: u64) -> Result<()> {
    let reference = reth_witness(reference_path)?;
    let have: std::collections::HashSet<&[u8]> =
        reference.state.iter().map(|b| b.as_ref()).collect();
    let missing = rebuilt
        .state
        .iter()
        .filter(|n| !have.contains(n.as_ref()))
        .count();
    if missing > 0 {
        bail!(
            "block {block}: {missing}/{} rebuilt trie nodes are absent from {} — \
             the reconstruction disagrees with the real witness",
            rebuilt.state.len(),
            reference_path.display()
        );
    }
    info!(
        "block {block}: all {} rebuilt nodes present in the reference witness ({} nodes)",
        rebuilt.state.len(),
        reference.state.len()
    );
    Ok(())
}

/// Locate the reth `.bin` for `block` — either the given file, or a directory
/// entry whose name carries that block number.
fn reference_for(path: &Path, block: u64) -> Result<PathBuf> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    let needle = format!("_{block}_");
    fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(&needle) && n.ends_with(".bin"))
        })
        .ok_or_else(|| anyhow!("no reth input for block {block} under {}", path.display()))
}

/// Decode, rebuild the pre-state trie, and verify it against the anchor the
/// container carries — then, if `emit` is set, fetch the header and write the
/// reth input. The verify step is unconditional: a reconstruction that does not
/// hash back to the parent state root never reaches the emit stage.
async fn process<P: Provider>(
    in_path: &Path,
    compare_reth: Option<&Path>,
    emit: Option<&(P, PathBuf)>,
    chain: &str,
) -> Result<()> {
    let bytes = fs::read(in_path).with_context(|| format!("reading {}", in_path.display()))?;
    let zeg = zeg0::parse(&bytes).context("parsing the ZEG0 container")?;
    let block = zeg.consensus.number;

    let rebuilt =
        zeg0::rebuild(&zeg.trie_stream, zeg.version).context("rebuilding the pre-state trie")?;
    zeg0::check_root(&rebuilt, zeg.consensus.parent_state_root)?;
    info!(
        "block {block} — {} txs, {} codes, {} ancestors, {} trie nodes, {} preimages, root {} verified",
        zeg.transactions.len(),
        zeg.codes.len(),
        zeg.prev_blocks.len(),
        rebuilt.state.len(),
        rebuilt.keys.len(),
        rebuilt.root
    );

    if let Some(dir) = compare_reth {
        compare(&rebuilt, &reference_for(dir, block)?, block)?;
    }

    let Some((provider, out_dir)) = emit else {
        return Ok(());
    };

    let header = fetch_header(provider, block).await?;
    let out_path = out_dir.join(format!(
        "{}_{}_{}_{}_zec_reth.bin",
        chain.to_lowercase(),
        block,
        zeg.transactions.len(),
        header.gas_used / 1_000_000,
    ));

    let stateless = assemble::build_stateless_input(&zeg, header, rebuilt.state, rebuilt.keys)?;
    RethClient
        .from_stateless_input(&stateless)
        .context("building the reth ZiskStdin")?
        .save(&out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;

    info!("ok  {} -> {}", in_path.display(), out_path.display());
    Ok(())
}

/// Fetch just the header. Deliberately not `debug_executionWitness` — that is
/// the archive-only call this tool exists to avoid.
async fn fetch_header<P: Provider>(provider: &P, number: u64) -> Result<Header> {
    let block = provider
        .get_block_by_number(number.into())
        .await
        .with_context(|| format!("eth_getBlockByNumber for block {number}"))?
        .ok_or_else(|| anyhow!("RPC has no block {number}"))?;
    Ok(block.header.inner)
}

//! Offline transcoder: reth stateless-validator `.bin` → ethrex input `.bin`.
//!
//! Both describe the same execution: a block plus its `debug_executionWitness`.
//! The ethrex input is the block and that witness, which `input-gen` would
//! otherwise fetch over RPC (the witness needs an archive node), so a reth
//! input carries everything it needs.

use std::fs;
use std::path::{Path, PathBuf};

use alloy_rlp::Encodable;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;

use ethrex_common::types::block_execution_witness::{decode_witness_headers, RpcExecutionWitness};
use ethrex_common::types::Block;
use ethrex_common::NativeCrypto;
use ethrex_config::networks::Network;
use ethrex_rlp::decode::RLPDecode;
use guest_ethrex::EthrexInput;
use input_reth::guest::{RethInputPublic, RethInputWitness};
use zisk_sdk::ZiskStdin;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Transcode reth stateless-validator .bin files into ethrex inputs (offline, no RPC)"
)]
struct Cli {
    /// A reth `.bin` file, or a directory of them.
    input: PathBuf,

    /// Output directory for the generated `*_zec_ethrex.bin` files.
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

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
        match convert_file(in_path, &cli.output_dir) {
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

fn convert_file(in_path: &Path, out_dir: &Path) -> Result<PathBuf> {
    let stdin = ZiskStdin::from_file(in_path)?;
    let public: RethInputPublic = stdin.read().context("reading the public frame")?;
    let witness = stdin
        .read::<RethInputWitness>()
        .context("reading the witness frame")?
        .witness;

    // The block, through its RLP.
    let mut rlp = Vec::new();
    public.block.encode(&mut rlp);
    let block = Block::decode(&rlp).map_err(|e| anyhow!("decoding the block for ethrex: {e}"))?;
    let number = block.header.number;

    // The same chain config `input-gen` uses: ethrex's built-in genesis for the chain id.
    let chain_id = public.chain_config.chain_id;
    let chain_config = Network::try_from(chain_id)
        .map_err(|e| anyhow!("unsupported chain id {chain_id}: {e}"))?
        .get_genesis()
        .context("loading the genesis config")?
        .config;

    let rpc_witness = RpcExecutionWitness {
        state: witness.state.into_iter().map(|b| b.0).collect(),
        keys: witness.keys.into_iter().map(|b| b.0).collect(),
        codes: witness.codes.into_iter().map(|b| b.0).collect(),
        headers: witness.headers.into_iter().map(|b| b.0).collect(),
    };
    let headers = decode_witness_headers(&rpc_witness.headers)
        .map_err(|e| anyhow!("decoding witness headers: {e}"))?;
    let execution_witness = rpc_witness
        .into_execution_witness(chain_config, number, &headers, &NativeCrypto)
        .map_err(|e| anyhow!("converting the execution witness: {e}"))?;

    let out_path = out_dir.join(format!(
        "{}_{}_{}_{}_zec_ethrex.bin",
        Network::try_from(chain_id)
            .map(|n| n.to_string())
            .unwrap_or_else(|_| "mainnet".into()),
        number,
        block.body.transactions.len(),
        block.header.gas_used / 1_000_000,
    ));

    let bytes = EthrexInput::new(block, execution_witness)
        .serialize()
        .map_err(|e| anyhow!("serializing the ethrex input: {e}"))?;
    let out = ZiskStdin::new();
    out.write_slice(&bytes);
    out.save(&out_path)
        .with_context(|| format!("write {}", out_path.display()))?;
    Ok(out_path)
}

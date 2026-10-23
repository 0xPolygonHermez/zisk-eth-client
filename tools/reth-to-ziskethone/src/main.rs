//! Offline transcoder: reth stateless-validator `.bin` → ziskethone `ZEG0`
//! input `.bin`, without any RPC.
//!
//! The reth and ziskethone inputs describe the *same* block execution: a block
//! plus its `debug_executionWitness` (state trie nodes, codes, keys, ancestor
//! headers). A committed reth `.bin` carries all of that, which lets us
//! regenerate ziskethone inputs for blocks too old for a normal RPC to serve
//! state.
//!
//! The one thing it doesn't carry is the set of accounts and slots the block
//! accesses. ZEG0 needs every one of them as a trie leaf, including keys that
//! don't exist (the guest halts on any other access), and reth's
//! `witness.keys` omits accounts that never existed. Online, the prestate
//! tracer supplies them; here we re-execute the block and record every
//! account and slot it accesses.
//!
//! Pipeline:
//!   1. Read the reth `.bin`'s two ZiskStdin frames (public, witness).
//!   2. Validate the block statelessly with reth.
//!   3. Re-execute it under `RecordingDb` + `SlotRecorder` → accessed keys.
//!   4. `rust_input_gen::assemble::sources_from_parts` → `OfflineSources`
//!      (parent + ancestors from `witness.headers`, compiled-in fork schedule).
//!   5. `encode_binary` → ZEG0 bytes → one ZiskStdin frame, matching what the
//!      ziskethone client writes.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use alloy_consensus::proofs::calculate_receipt_root;
use alloy_consensus::{Header, TxReceipt};
use alloy_eips::eip2718::{Decodable2718, Encodable2718};
use alloy_primitives::{keccak256, map::B256IndexMap, Address, Bloom, B256, U256};
use input_reth::guest::{
    get_chain_spec, validate_block_stateless, RethInputPublic, RethInputWitness,
};
use reth_ethereum_primitives::Block;
use reth_evm::block::BlockExecutor;
use reth_evm::ConfigureEvm;
use reth_evm_ethereum::EthEvmConfig;
use reth_trie_common::{HashedPostState, KeccakKeyHasher};
use revm::bytecode::{opcode, Bytecode};
use revm::database::states::bundle_state::BundleRetention;
use revm::database::State;
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::interpreter_types::Jumps;
use revm::interpreter::Interpreter;
use revm::state::AccountInfo;
use revm::{Database, Inspector};
use rust_input_gen::assemble::{rpc_block, sources_from_parts};
use rust_input_gen::offline::{encode_binary, OfflineSources};
use rust_input_gen::state_root::Accessed;
use stateless_reth::{recover_block_with_public_keys, ExecutionWitness, StatelessTrie};
use stateless_reth_tries::zeth::SparseState;
use stateless_reth_tries::WitnessDbError;
use zisk_sdk::ZiskStdin;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Transcode reth stateless-validator .bin files into ziskethone ZEG0 inputs (offline, no RPC)"
)]
struct Cli {
    /// A reth `.bin` file, or a directory of them.
    input: PathBuf,

    /// Output directory for the generated `*_zec_ziskethone.bin` files.
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Chain slug used in the output filename (these inputs are mainnet).
    #[arg(long, default_value = "mainnet")]
    chain: String,

    /// Keep the input file's own name for the output, instead of deriving
    /// `<chain>_<block>_<txs>_<mgas>_zec_ziskethone.bin` from block metadata.
    /// Required for corpora whose blocks share metadata — every EEST fixture
    /// is block 1, so the derived name collides across the whole corpus.
    #[arg(long)]
    keep_name: bool,

    /// How to resolve the fork. `auto` reads the block timestamp against the
    /// compiled-in schedule for the input's chain id. `eest-osaka` forces Osaka
    /// with the EEST base-Osaka blob schedule, for fixtures whose synthetic
    /// timestamps (12, 24, ...) carry no mainnet meaning.
    #[arg(long, value_enum, default_value_t = ForkMode::Auto)]
    fork: ForkMode,

    /// Compute the header's execution outputs (state_root, receipts_root,
    /// logs_bloom, gas_used) by executing the block, before validating it. For
    /// inputs from `ziskethone-to-reth --offline`, which leaves them zero.
    /// Validation still checks everything else, but no longer that the block
    /// matches the canonical chain.
    #[arg(long)]
    fill_outputs: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
enum ForkMode {
    /// Derive the fork from the block timestamp via the chain's schedule.
    Auto,
    /// Force Osaka + the EEST base-Osaka blob schedule.
    EestOsaka,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

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
    // Two inputs must never resolve to one output path: the metadata-derived
    // name collides for any corpus of same-shaped blocks, and a silent
    // overwrite would look like a clean run that dropped most of the corpus.
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    for in_path in &inputs {
        match convert_file(in_path, &cli, &mut claimed) {
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

fn convert_file(in_path: &Path, cli: &Cli, claimed: &mut HashSet<PathBuf>) -> Result<PathBuf> {
    let stdin = ZiskStdin::from_file(in_path)?;
    let public: RethInputPublic = stdin.read().context("reading the public frame")?;
    let witness = stdin
        .read::<RethInputWitness>()
        .context("reading the witness frame")?
        .witness;

    let (number, tx_count) = (
        public.block.header.number,
        public.block.body.transactions.len(),
    );

    // `gas_used` is returned rather than read here: --fill-outputs computes it.
    let (sources, gas_used) = build_sources(public, witness, cli.fork, cli.fill_outputs)
        .context("assemble OfflineSources")?;
    let zeg0 = encode_binary(&sources).context("encode ZEG0 container")?;

    let filename = if cli.keep_name {
        let stem = in_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("input {} has no usable file stem", in_path.display()))?;
        format!("{stem}.bin")
    } else {
        format!(
            "{}_{}_{}_{}_zec_ziskethone.bin",
            cli.chain.to_lowercase(),
            number,
            tx_count,
            gas_used / 1_000_000,
        )
    };
    let out_path = cli.output_dir.join(filename);
    if !claimed.insert(out_path.clone()) {
        bail!(
            "output {} already written by an earlier input -- name collision; \
             re-run with --keep-name to name outputs after their input files",
            out_path.display()
        );
    }

    let out = ZiskStdin::new();
    out.write_slice(&zeg0);
    out.save(&out_path)
        .with_context(|| format!("write {}", out_path.display()))?;
    Ok(out_path)
}

fn build_sources(
    public: RethInputPublic,
    witness: ExecutionWitness,
    fork: ForkMode,
    fill_outputs: bool,
) -> Result<(OfflineSources, u64)> {
    let chain_id = public.chain_config.chain_id;
    let (block, senders, accessed) =
        accessed_keys(public.block.clone(), &public, witness.clone(), fill_outputs)?;
    let gas_used = block.header.gas_used;

    // reth's transactions → alloy envelopes, through their EIP-2718 bytes.
    let Block { header, body } = block;
    let txs = body
        .transactions
        .iter()
        .zip(senders)
        .map(|(tx, sender)| {
            let env = alloy_consensus::TxEnvelope::decode_2718(&mut tx.encoded_2718().as_slice())
                .map_err(|e| anyhow!("re-decoding a transaction envelope: {e}"))?;
            Ok((env, sender))
        })
        .collect::<Result<Vec<_>>>()?;
    let current = rpc_block(header, txs, body.withdrawals.map(|w| w.into_inner()));

    // The submodule's witness format and reth's are field-identical.
    let witness = rust_input_gen::rpc::ExecutionWitness {
        state: witness.state,
        codes: witness.codes,
        keys: witness.keys,
        headers: witness.headers,
    };
    let mut sources = sources_from_parts(chain_id, current, witness, accessed)?;

    if fork == ForkMode::EestOsaka {
        // EEST base-Osaka. The fixtures' timestamps are synthetic (12, 24, ...)
        // so the mainnet schedule would read them as pre-Cancun and, since the
        // headers are Prague-shaped, emit fork_id = FORK_PRAGUE -- running
        // Osaka tests under Prague EVM rules (wrong MODEXP gas, no CLZ, no
        // P256VERIFY). Fraction 5_007_716 is EEST base-Osaka's, per
        // eest-witness-gen's manifest; target/max stay 0 so the guest skips
        // its excess_blob_gas re-derivation, matching what that canonical
        // manifest path emits (it carries no target/max fields at all).
        sources.is_osaka = true;
        sources.blob_base_fee_update_fraction = 5_007_716;
        sources.target_blob_gas_per_block = 0;
        sources.max_blob_gas_per_block = 0;
    }
    Ok((sources, gas_used))
}

/// Validate `block` statelessly, then re-execute it recording every account
/// and slot it accesses. With `fill_outputs`, execute first and write the
/// execution outputs into the header before validating. Returns the block (as
/// validated), the transaction senders and the accessed keys.
fn accessed_keys(
    mut block: Block,
    public: &RethInputPublic,
    witness: ExecutionWitness,
    fill_outputs: bool,
) -> Result<(Block, Vec<Address>, Accessed)> {
    let chain_spec = get_chain_spec(&public.chain_config);
    let recover = |block: Block| {
        recover_block_with_public_keys(block, public.public_keys.clone(), &*chain_spec)
            .map_err(|e| anyhow!("recovering transaction senders: {e}"))
    };
    let validate = |recovered| {
        validate_block_stateless(recovered, witness.clone(), chain_spec.clone())
            .map_err(|e| anyhow!("reth stateless validation failed: {e}"))
    };
    let recovered = recover(block.clone())?;
    if !fill_outputs {
        validate(recovered.clone())?;
    }

    // Ancestor hashes for BLOCKHASH, and the parent's state root.
    let mut block_hashes = BTreeMap::new();
    let mut parent_state_root = None;
    for raw in &witness.headers {
        let header: Header = alloy_rlp::decode_exact(raw.as_ref())
            .map_err(|e| anyhow!("decoding an ancestor header: {e}"))?;
        let hash = keccak256(raw);
        if hash == block.header.parent_hash {
            parent_state_root = Some(header.state_root);
        }
        block_hashes.insert(header.number, hash);
    }
    let parent_state_root = parent_state_root.context("parent header not in witness.headers")?;

    let (mut trie, bytecode) = SparseState::new(&witness, parent_state_root)
        .map_err(|e| anyhow!("building the pre-state trie: {e}"))?;
    let mut state = State::builder()
        .with_database(RecordingDb {
            trie: &trie,
            bytecode,
            block_hashes,
            accessed: Accessed::new(),
        })
        .with_bundle_update()
        .build();

    // The database sees every account and every slot revm loads from it; the
    // inspector adds the SLOADs/SSTOREs revm answers itself — the storage of a
    // contract created in this block, which revm knows starts empty — since the
    // guest needs every slot it loads as a leaf.
    let mut recorder = SlotRecorder::default();
    let evm_config = EthEvmConfig::new(chain_spec.clone());
    let evm_env = evm_config
        .evm_env(recovered.header())
        .map_err(|e| anyhow!("{e}"))?;
    let evm = evm_config.evm_with_env_and_inspector(&mut state, evm_env, &mut recorder);
    let ctx = evm_config
        .context_for_block(recovered.sealed_block())
        .map_err(|e| anyhow!("{e}"))?;
    let result = evm_config
        .create_executor(evm, ctx)
        .execute_block(recovered.transactions_recovered())
        .map_err(|e| anyhow!("re-execution failed: {e}"))?;

    let mut accessed = std::mem::take(&mut state.database.accessed);
    for (addr, slots) in recorder.0 {
        accessed.entry(addr).or_default().extend(slots);
    }

    if fill_outputs {
        // The same computations reth's post-execution validation compares.
        let receipts: Vec<_> = result
            .receipts
            .iter()
            .map(TxReceipt::with_bloom_ref)
            .collect();
        // execute_block leaves changes in the transition state; Executor::execute
        // merges them the same way before taking the bundle.
        state.merge_transitions(BundleRetention::Reverts);
        let bundle = state.take_bundle();
        drop(state);
        let hashed = HashedPostState::from_bundle_state::<KeccakKeyHasher>(&bundle.state);
        let header = &mut block.header;
        header.gas_used = result.gas_used;
        header.receipts_root = calculate_receipt_root(&receipts);
        header.logs_bloom = receipts
            .iter()
            .fold(Bloom::ZERO, |bloom, r| bloom | r.bloom_ref());
        header.state_root = trie
            .calculate_state_root(hashed)
            .map_err(|e| anyhow!("computing the post-state root: {e}"))?;
        validate(recover(block.clone())?)?;
        // Matches the canonical hash only if every filled field is right.
        println!(
            "filled block {} hash {}",
            block.header.number,
            block.header.hash_slow()
        );
    }
    Ok((block, recovered.senders().to_vec(), accessed))
}

/// Records the (address, slot) of every SLOAD and SSTORE.
#[derive(Default)]
struct SlotRecorder(Accessed);

impl<CTX> Inspector<CTX, EthInterpreter> for SlotRecorder {
    fn step(&mut self, interp: &mut Interpreter<EthInterpreter>, _context: &mut CTX) {
        let op = interp.bytecode.opcode();
        if op != opcode::SLOAD && op != opcode::SSTORE {
            return;
        }
        // The slot is on top of the stack for both.
        if let Ok(slot) = interp.stack.peek(0) {
            self.0
                .entry(interp.input.target_address)
                .or_default()
                .insert(B256::from(slot));
        }
    }
}

/// revm `Database` over the witness trie that records every key it serves.
/// Otherwise the stateless crate's own `WitnessDatabase`, which is private.
#[derive(Debug)]
struct RecordingDb<'a> {
    trie: &'a SparseState,
    bytecode: B256IndexMap<Bytecode>,
    block_hashes: BTreeMap<u64, B256>,
    accessed: Accessed,
}

impl Database for RecordingDb<'_> {
    type Error = WitnessDbError;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.accessed.entry(address).or_default();
        Ok(self.trie.account(address)?.map(|account| AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
            code: None,
            account_id: None,
        }))
    }

    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        self.accessed
            .entry(address)
            .or_default()
            .insert(B256::from(slot));
        self.trie.storage(address, slot)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.bytecode.get(&code_hash).cloned().ok_or_else(|| {
            WitnessDbError::TrieWitness(format!("bytecode for {code_hash} not found"))
        })
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        self.block_hashes
            .get(&number)
            .copied()
            .ok_or(WitnessDbError::StateNotFound(number))
    }
}

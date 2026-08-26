use anyhow::{Context, Result};
use async_trait::async_trait;
use zisk_sdk::ZiskStdin;

/// Headers are honored only by clients that support custom HTTP headers
/// (currently `reth`). Others warn once and ignore them.
#[derive(Debug, Clone, Default)]
pub struct RpcConfig {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

impl RpcConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }
}

/// Parse a `Key:Value` header pair (used by clap as a `value_parser`).
pub fn parse_header(s: &str) -> Result<(String, String)> {
    let (k, v) = s
        .split_once(':')
        .with_context(|| format!("Invalid header (expected `Key:Value`): {s}"))?;
    Ok((k.trim().to_string(), v.trim().to_string()))
}

/// Block-level facts the host can recover from an input file on its own, with
/// no guest run. Unlike [`BlockStats`] it carries no chain identity: not every
/// input format embeds the chain config.
#[derive(Debug, Clone, Copy)]
pub struct InputStats {
    pub block_number: u64,
    pub tx_count: usize,
    pub gas_used: u64,
}

/// Borrow the first length-prefixed frame of a raw `ZiskStdin` buffer.
///
/// Frames are laid out by `ZiskStdin::write_slice`: an 8-byte little-endian
/// payload length, the payload, then padding to an 8-byte boundary.
pub fn first_frame(buf: &[u8]) -> Result<&[u8]> {
    let len_bytes: [u8; 8] = buf
        .get(..8)
        .and_then(|s| s.try_into().ok())
        .context("input is too short to hold a frame length prefix")?;
    let len = u64::from_le_bytes(len_bytes) as usize;
    buf.get(8..8 + len)
        .with_context(|| format!("input is truncated: first frame claims {len} bytes"))
}

#[derive(Debug, Clone)]
pub struct BlockStats {
    pub chain_name: &'static str,
    pub block_number: u64,
    pub tx_count: usize,
    pub gas_used: u64,
}

impl BlockStats {
    /// `<chain>_<block>_<txs>_<mgas>_zec_<client>.bin`
    pub fn output_filename(&self, client_name: &str) -> String {
        format!(
            "{}_{}_{}_{}_zec_{}.bin",
            self.chain_name.to_lowercase(),
            self.block_number,
            self.tx_count,
            self.gas_used / 1_000_000,
            client_name,
        )
    }
}

#[async_trait]
pub trait ExecutionClient: Send + Sync {
    /// Slug used for output filenames and the default `<client>-inputs` output
    /// dir (e.g. `"reth"`). Not the `--client` flag — that comes from the
    /// `Client` enum variant via `clap::ValueEnum`.
    fn name(&self) -> &'static str;

    /// Display name used in logs (e.g. `"Reth"`).
    fn display_name(&self) -> &'static str;

    // `from_rpc` takes `&self` (it dispatches on the client instance), not the
    // associated-fn form clippy's `wrong_self_convention` expects for `from_*`.
    #[allow(clippy::wrong_self_convention)]
    async fn from_rpc(
        &self,
        config: &RpcConfig,
        block_number: u64,
    ) -> Result<(ZiskStdin, BlockStats)>;

    fn run(&self);

    /// Decode the block-level stats out of an input built by
    /// [`from_rpc`](Self::from_rpc), so a benchmark can report tx count and gas
    /// without the guest reporting them back. `None` for clients whose input
    /// format the host cannot decode.
    fn input_stats(&self, _stdin: &ZiskStdin) -> Result<Option<InputStats>> {
        Ok(None)
    }

    /// Whether [`run`](Self::run) emits ZisK hints. `true` for instrumented
    /// guest runs (reth, ethrex); `false` for native-only clients like
    /// `ziskethone` whose `run()` is a C++ input checker. The hints harness
    /// rejects `false` clients rather than writing an empty hints file.
    fn emits_hints(&self) -> bool {
        true
    }
}

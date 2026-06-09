#![allow(clippy::wrong_self_convention)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use zisk_sdk::ZiskStdin;

use crate::{EthrexClient, RethClient};

/// Execution clients. To add one, see `docs/adding-a-client.md` — adding a
/// variant here makes `create_client` (and the host's ELF/input-gen matches)
/// fail to compile until you wire up the new client.
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Reth,
    Ethrex,
}

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

pub fn create_client(client: Client) -> Box<dyn ExecutionClient> {
    match client {
        Client::Reth => Box::new(RethClient),
        Client::Ethrex => Box::new(EthrexClient),
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

    async fn from_rpc(
        &self,
        config: &RpcConfig,
        block_number: u64,
    ) -> Result<(ZiskStdin, BlockStats)>;

    fn run(&self);
}

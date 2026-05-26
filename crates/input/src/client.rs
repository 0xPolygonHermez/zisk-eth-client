use anyhow::Result;
use async_trait::async_trait;
use zisk_sdk::ZiskStdin;

use crate::{EthrexClient, RethClient};

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Reth,
    Ethrex,
}

/// Per-block metadata returned alongside the generated [`ZiskStdin`] by
/// [`ExecutionClient::from_rpc`]. Callers use it to derive output filenames
/// and log lines.
#[derive(Debug, Clone)]
pub struct BlockStats {
    pub chain_name: &'static str,
    pub block_number: u64,
    pub tx_count: usize,
    pub gas_used: u64,
}

impl BlockStats {
    /// Canonical output filename: `<chain>_<block>_<txs>_<mgas>_zec_<client>.bin`.
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
    /// Short identifier used for filenames and CLI flags (e.g. `"reth"`).
    fn name(&self) -> &'static str;

    /// Human-readable name used in logs (e.g. `"Reth"`).
    fn display_name(&self) -> &'static str;

    /// Generate `ZiskStdin` from a live RPC endpoint, alongside block metadata.
    async fn from_rpc(&self, rpc_url: &str, block_number: u64) -> Result<(ZiskStdin, BlockStats)>;

    /// Run native guest execution (used for hints generation).
    fn run(&self);
}

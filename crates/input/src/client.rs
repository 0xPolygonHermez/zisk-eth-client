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

/// Map a chain ID to a display name. Returns `"Unknown"` for unsupported chains.
pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Mainnet",
        11155111 => "Sepolia",
        17000 => "Holesky",
        560048 => "Hoodi",
        _ => "Unknown",
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
    fn name(&self) -> &'static str;

    fn display_name(&self) -> &'static str {
        self.name()
    }

    /// Generate `ZiskStdin` from a live RPC endpoint, alongside block metadata.
    async fn from_rpc(&self, rpc_url: &str, block_number: u64) -> Result<(ZiskStdin, BlockStats)>;

    /// Run native guest execution (used for hints generation).
    fn run(&self);
}

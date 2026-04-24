use anyhow::Result;
use async_trait::async_trait;
use stateless_reth::StatelessInput;
use zisk_sdk::ZiskStdin;

use crate::{EthrexClient, RethClient};

#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Reth,
    Ethrex,
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

    /// Generate ZiskStdin from a live RPC endpoint.
    async fn from_rpc(&self, rpc_url: &str, block_number: u64) -> Result<ZiskStdin>;

    /// Generate ZiskStdin from a pre-fetched StatelessInput (reth only by default).
    fn from_stateless_input(&self, _input: &StatelessInput) -> Result<ZiskStdin> {
        anyhow::bail!("{} does not support StatelessInput", self.name())
    }

    /// Run native guest execution (used for hints generation).
    fn run(&self);
}

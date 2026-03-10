use clap::ValueEnum;
use std::path::Path;

use witness_generator::StatelessValidationFixture;

mod reth;

/// Available clients for CLI selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Client {
    /// Reth execution client
    Reth,
    // Add more clients here as needed
}

/// Trait for execution clients that generate zkVM inputs
pub trait ExecutionClient: Send + Sync {
    /// Human-readable name for this client
    fn name(&self) -> &'static str;

    /// Display name for this client (used in logs and messages)
    fn display_name(&self) -> &'static str {
        self.name()
    }

    /// Process a fixture: generate input and save to output directory
    fn process_fixture(
        &self,
        fixture: &StatelessValidationFixture,
        output_dir: &Path,
    ) -> Result<(), anyhow::Error>;
}

/// Factory function to create an execution client
pub fn create_client(client: &Client) -> Box<dyn ExecutionClient> {
    match client {
        Client::Reth => Box::new(reth::RethClient::new()),
    }
}

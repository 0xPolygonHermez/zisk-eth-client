mod reth;

use clap::ValueEnum;

use witness_generator::StatelessValidationFixture;

use crate::processor::ProcessingResult;

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

    /// Generate input from a fixture
    fn generate_input(
        &self,
        fixture: &StatelessValidationFixture,
    ) -> Result<ProcessingResult, anyhow::Error>;
}

/// Factory function to create an execution client
pub fn create_client(client: &Client) -> Box<dyn ExecutionClient> {
    match client {
        Client::Reth => Box::new(reth::RethClient::new()),
    }
}

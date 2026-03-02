pub mod ethrex;
pub mod reth;

use clap::ValueEnum;

use witness_generator::StatelessValidationFixture;

use crate::processor::ProcessingResult;

/// Available client types for CLI selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClientType {
    /// Reth execution client
    Reth,
    /// Ethrex execution client
    Ethrex,
}

/// Trait for execution clients that generate zkVM inputs
pub trait ExecutionClient: Send + Sync {
    /// Human-readable name for this client
    fn name(&self) -> &'static str;

    /// The client type variant
    fn client_type(&self) -> ClientType;

    /// Generate input from a fixture
    fn generate_input(
        &self,
        fixture: &StatelessValidationFixture,
    ) -> Result<ProcessingResult, anyhow::Error>;
}

/// Factory function to create an execution client
pub fn create_client(client_type: &ClientType) -> Box<dyn ExecutionClient> {
    match client_type {
        ClientType::Reth => Box::new(reth::RethClient::new()),
        ClientType::Ethrex => Box::new(ethrex::EthrexClient::new()),
    }
}

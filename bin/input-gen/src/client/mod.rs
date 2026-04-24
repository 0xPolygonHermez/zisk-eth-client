use std::path::Path;

use anyhow::Result;
use witness_generator::StatelessValidationFixture;

use crate::provider::ProviderKind;

mod ethrex;
mod reth;

pub use input::Client;

/// Extends the core ExecutionClient with input-gen-specific capabilities:
/// provider compatibility and fixture processing.
pub trait ExecutionClient: input::ExecutionClient {
    /// Which provider types this client supports
    fn supported_providers(&self) -> &'static [ProviderKind];

    /// Check if this client supports a given provider type
    fn supports_provider(&self, provider: ProviderKind) -> bool {
        self.supported_providers().contains(&provider)
    }

    /// Process from a fixture: generate input and save to output directory
    fn process_fixture(
        &self,
        fixture: &StatelessValidationFixture,
        output_dir: &Path,
    ) -> Result<()> {
        let stdin = self.from_stateless_input(&fixture.stateless_input)?;
        let filename = sanitize_filename(&fixture.name);
        let output_path = output_dir.join(format!("{}.bin", filename));
        stdin.save(&output_path)?;
        Ok(())
    }
}

pub fn create_client(client: Client) -> Box<dyn ExecutionClient> {
    match client {
        Client::Reth => Box::new(input::RethClient::default()),
        Client::Ethrex => Box::new(input::EthrexClient::default()),
    }
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

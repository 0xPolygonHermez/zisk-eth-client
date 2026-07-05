use std::path::Path;

use anyhow::Result;
use witness_generator::StatelessValidationFixture;

use crate::input_gen::provider::ProviderKind;

mod ethrex;
mod reth;
#[cfg(feature = "ziskethone")]
mod ziskethone;

pub use input::Client;

/// Extends [`input::ExecutionClient`] with input-gen-specific capabilities:
/// provider compatibility and fixture processing.
///
/// Clients that don't support fixture-based providers (e.g. EEST) get the
/// default-bail `process_fixture` impl. The `supports_provider` check is the
/// real guard — callers should consult it before invoking `process_fixture`.
pub trait InputGenClient: input::ExecutionClient {
    /// Which provider types this client supports.
    fn supported_providers(&self) -> &'static [ProviderKind];

    /// Check if this client supports a given provider type.
    fn supports_provider(&self, provider: ProviderKind) -> bool {
        self.supported_providers().contains(&provider)
    }

    /// Process a fixture: generate input and save to output directory.
    ///
    /// Default impl bails — only clients that support fixture-based providers
    /// (i.e. those listing `ProviderKind::Eest` in `supported_providers`) need
    /// to override this.
    fn process_fixture(
        &self,
        _fixture: &StatelessValidationFixture,
        _output_dir: &Path,
    ) -> Result<()> {
        anyhow::bail!(
            "{} does not support fixture-based input generation",
            self.name()
        )
    }
}

// To add a client, see docs/adding-a-client.md.
pub fn create_client(client: Client) -> Box<dyn InputGenClient> {
    match client {
        Client::Reth => Box::new(input::RethClient),
        Client::Ethrex => Box::new(input::EthrexClient),
        #[cfg(feature = "ziskethone")]
        Client::ZiskEthOne => Box::new(input::ZiskEthOneClient),
    }
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

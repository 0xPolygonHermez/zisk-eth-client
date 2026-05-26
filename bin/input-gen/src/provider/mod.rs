pub mod eest;
pub mod rpc;

use crate::client::InputGenClient;
use std::path::Path;

/// Provider type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Eest,
    Rpc,
}

/// Common trait for all input providers
#[async_trait::async_trait]
pub trait InputProvider: Send + Sync {
    /// Identifier for the provider type
    fn kind(&self) -> ProviderKind;

    /// Generate ZisK inputs for the given client
    async fn generate_inputs(
        &self,
        client: &dyn InputGenClient,
        output: &Path,
    ) -> anyhow::Result<()>;
}

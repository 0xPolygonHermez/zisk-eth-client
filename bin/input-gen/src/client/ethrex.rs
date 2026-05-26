use super::ExecutionClient;
use crate::provider::ProviderKind;

impl ExecutionClient for input::EthrexClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Rpc]
    }
}

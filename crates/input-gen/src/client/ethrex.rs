use super::InputGenClient;
use crate::provider::ProviderKind;

impl InputGenClient for input::EthrexClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Rpc]
    }
}

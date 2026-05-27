use super::InputGenClient;
use crate::input_gen::provider::ProviderKind;

impl InputGenClient for input::EthrexClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Rpc]
    }
}

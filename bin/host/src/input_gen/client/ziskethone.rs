use super::InputGenClient;
use crate::input_gen::provider::ProviderKind;

// ziskethone has no EEST fixture format; RPC only.
impl InputGenClient for input::ZiskEthOneClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Rpc]
    }
}

use super::InputGenClient;
use crate::input_gen::provider::ProviderKind;

// zilkworm has no EEST fixture format; RPC only.
impl InputGenClient for input::ZilkwormClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Rpc]
    }
}

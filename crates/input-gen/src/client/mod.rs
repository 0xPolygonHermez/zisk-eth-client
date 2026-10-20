use std::path::Path;

use crate::common::StatelessFixture;
use anyhow::Result;

use crate::provider::ProviderKind;

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
    fn process_fixture(&self, _fixture: &StatelessFixture, _output_dir: &Path) -> Result<()> {
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

/// Maximum length of a generated file stem. EEST test ids run past the
/// 255-byte file name limit of most filesystems; longer names are cut and
/// suffixed with a hash of the full name so they stay unique.
const MAX_STEM_LEN: usize = 180;

pub(crate) fn sanitize_filename(name: &str) -> String {
    let sanitized = name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    if sanitized.len() <= MAX_STEM_LEN {
        return sanitized;
    }
    let digest = alloy_primitives::keccak256(name.as_bytes());
    let suffix = format!("~{}", alloy_primitives::hex::encode(&digest[..6]));
    let mut head = sanitized;
    let mut cut = MAX_STEM_LEN - suffix.len();
    while !head.is_char_boundary(cut) {
        cut -= 1;
    }
    head.truncate(cut);
    head + &suffix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_names_are_shortened_and_unique() {
        let base = "tests/amsterdam/x.py::test[".to_string() + &"a".repeat(400) + "]#block0";
        let other = base.replace("]#block0", "]#block1");
        let (a, b) = (sanitize_filename(&base), sanitize_filename(&other));
        assert!(a.len() <= MAX_STEM_LEN && b.len() <= MAX_STEM_LEN);
        assert_ne!(a, b);
        assert!(a.starts_with("tests_amsterdam_x.py__test["));
        assert_eq!(sanitize_filename("short/name:1"), "short_name_1");
    }
}

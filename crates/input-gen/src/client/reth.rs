use std::path::Path;

use anyhow::{Context, Result};
use zisk_sdk::ZiskStdin;

use super::{sanitize_filename, InputGenClient};
use crate::{common::StatelessFixture, provider::ProviderKind};

impl InputGenClient for input::RethClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Eest, ProviderKind::Rpc]
    }

    /// Writes `<name>.bin` (the guest stdin) and `<name>.output` (the expected
    /// guest output, for checking a run).
    fn process_fixture(&self, fixture: &StatelessFixture, output_dir: &Path) -> Result<()> {
        // Written verbatim, without the decode check of `stdin_from_bytes`:
        // some fixtures are deliberately malformed inputs the guest must
        // reject, and the `.output` sidecar carries that expected rejection.
        let stdin = ZiskStdin::new();
        stdin.write_slice(&fixture.stateless_input_bytes);
        // Fixture names contain dots (`test_x.py::...`), so build the file
        // names by hand instead of `with_extension`.
        let name = sanitize_filename(&fixture.name);
        stdin.save(output_dir.join(format!("{name}.bin")))?;
        std::fs::write(
            output_dir.join(format!("{name}.output")),
            &fixture.stateless_output_bytes,
        )
        .with_context(|| format!("Failed to write expected output for {}", fixture.name))?;
        Ok(())
    }
}

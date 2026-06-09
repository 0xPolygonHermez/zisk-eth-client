use std::path::Path;

use anyhow::Result;
use witness_generator::StatelessValidationFixture;

use super::{InputGenClient, sanitize_filename};
use crate::input_gen::provider::ProviderKind;

impl InputGenClient for input::RethClient {
    fn supported_providers(&self) -> &'static [ProviderKind] {
        &[ProviderKind::Eest, ProviderKind::Rpc]
    }

    fn process_fixture(
        &self,
        fixture: &StatelessValidationFixture,
        output_dir: &Path,
    ) -> Result<()> {
        let stdin = self.from_stateless_input(&fixture.stateless_input)?;
        let output_path = output_dir.join(format!("{}.bin", sanitize_filename(&fixture.name)));
        stdin.save(&output_path)?;
        Ok(())
    }
}

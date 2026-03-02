use anyhow::Context;

use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexInput, StatelessValidatorEthrexIo,
};
use stateless_validator_reth::guest::{StatelessValidatorRethInput, StatelessValidatorRethIo};

use ere_io::Io;

pub enum ProcessingResult {
    Reth(StatelessValidatorRethInput),
    Ethrex(StatelessValidatorEthrexInput),
}

impl ProcessingResult {
    pub fn save_to_file(
        &self,
        file_name: &str,
        output_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let filename = sanitize_filename(file_name);

        std::fs::create_dir_all(output_dir)?;

        let output_path = output_dir.join(format!("{}.bin", filename));

        let bytes = match self {
            ProcessingResult::Reth(input) => StatelessValidatorRethIo::serialize_input(input)
                .map_err(|e| anyhow::anyhow!("Failed to serialize reth input: {}", e))?,
            ProcessingResult::Ethrex(input) => {
                StatelessValidatorEthrexIo::serialize_input(input)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize ethrex input: {}", e))?
            }
        };

        std::fs::write(&output_path, bytes)
            .with_context(|| format!("Failed to write input to {}", output_path.display()))?;

        Ok(())
    }
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

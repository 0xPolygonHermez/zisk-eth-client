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
        match self {
            ProcessingResult::Reth(input) => {
                let filename = sanitize_filename(file_name);

                std::fs::create_dir_all(output_dir)?;

                // Define output path with sanitized filename
                let output_path = output_dir.join(format!("{}.bin", filename));

                // Serialize the reth input to binary format
                let bytes = StatelessValidatorRethIo::serialize_input(&input)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize reth input: {}", e))?;

                // Write the binary data to the output file
                std::fs::write(&output_path, bytes).with_context(|| {
                    format!("Failed to write reth input to {}", output_path.display())
                })?;

                Ok(())
            }
            ProcessingResult::Ethrex(input) => {
                let filename = sanitize_filename(file_name);

                std::fs::create_dir_all(output_dir)?;

                // Define output path with sanitized filename
                let output_path = output_dir.join(format!("{}.bin", filename));

                // Serialize the ethrex input to binary format
                let bytes = StatelessValidatorEthrexIo::serialize_input(&input)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize Ethrex input: {}", e))?;

                // Write the binary data to the output file
                std::fs::write(&output_path, bytes).with_context(|| {
                    format!("Failed to write ethrex input to {}", output_path.display())
                })?;

                Ok(())
            }
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

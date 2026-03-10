use zisk_sdk::{ZiskIO, ZiskStdin};

use guest_reth::{RethInput, RethInputPublic, RethInputWitness};

pub enum ProcessingResult {
    Reth(RethInput),
    // Add more variants here for other clients
}

impl ProcessingResult {
    pub fn save_to_file(
        &self,
        file_name: &str,
        output_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(output_dir)?;

        let filename = sanitize_filename(file_name);
        let output_path = output_dir.join(format!("{}.bin", filename));

        let zisk_stdin = ZiskStdin::new();

        match self {
            ProcessingResult::Reth(input) => {
                // Write public
                let public = RethInputPublic {
                    block: input.stateless_input.block.clone(),
                    chain_config: input.stateless_input.chain_config.clone(),
                    public_keys: input.public_keys.clone(),
                };
                let public_bytes = RethInputPublic::serialize(&public)?;
                zisk_stdin.write_slice(&public_bytes);

                // Write witness
                let witness = RethInputWitness {
                    witness: input.stateless_input.witness.clone(),
                };
                let witness_bytes = RethInputWitness::serialize(&witness)?;
                zisk_stdin.write_slice(&witness_bytes);
            }
        }

        zisk_stdin.save(&output_path)?;

        Ok(())
    }
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

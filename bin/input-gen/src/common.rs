use anyhow::{Context, Result};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::path::Path;
use walkdir::WalkDir;

use stateless_validator_reth::guest::StatelessValidatorRethInput;
use witness_generator::StatelessValidationFixture;

use input::StatelessValidatorRethInputNoPk;

use crate::types::OutputFormat;

/// Reads fixture JSON files from a directory.
pub fn fixtures_from_path(path: &Path) -> Result<Vec<StatelessValidationFixture>> {
    WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
        })
        .par_bridge()
        .map(|entry| {
            let content = std::fs::read(entry.path())?;
            let fixture: StatelessValidationFixture = serde_json::from_slice(&content)
                .with_context(|| format!("Failed to parse {}", entry.path().display()))?;
            Ok(fixture)
        })
        .collect()
}

/// Generate a reth input from a fixture.
pub fn reth_input_from_fixture(
    fixture: &StatelessValidationFixture,
) -> Result<StatelessValidatorRethInput> {
    let reth_input = StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
        .with_context(|| {
            format!(
                "Failed to create StatelessValidatorReth input for {}",
                fixture.name
            )
        })?;
    Ok(reth_input)
}

pub fn reth_input_to_file(
    reth_input: StatelessValidatorRethInput,
    file_name: &str,
    output_dir: &Path,
    format: OutputFormat,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    let extension = match format {
        OutputFormat::Binary => "bin",
        OutputFormat::Json => "json",
    };
    let filename = sanitize_filename(file_name);

    // Save public keys
    let pk_path = output_dir.join(format!("{}.pk.{}", filename, extension));
    let pk_bytes = match format {
        OutputFormat::Binary => bincode::serialize(&reth_input.public_keys)?,
        OutputFormat::Json => serde_json::to_vec_pretty(&reth_input.public_keys)?,
    };
    std::fs::write(&pk_path, &pk_bytes)
        .with_context(|| format!("Failed to write public keys to {}", pk_path.display()))?;

    // Save main input
    let main_input = StatelessValidatorRethInputNoPk {
        new_payload_request: reth_input.new_payload_request,
        witness: reth_input.witness,
        chain_config: reth_input.chain_config,
    };
    let main_path = output_dir.join(format!("{}.wtns.{}", filename, extension));
    let main_bytes = match format {
        OutputFormat::Binary => bincode::serialize(&main_input)?,
        OutputFormat::Json => serde_json::to_vec_pretty(&main_input)?,
    };
    std::fs::write(&main_path, &main_bytes)
        .with_context(|| format!("Failed to write main input to {}", main_path.display()))?;

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

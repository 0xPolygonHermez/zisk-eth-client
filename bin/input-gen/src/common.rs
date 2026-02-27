use anyhow::{Context, Result};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::path::Path;
use walkdir::WalkDir;

use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexInput, StatelessValidatorEthrexIo,
};
use stateless_validator_reth::guest::{StatelessValidatorRethInput, StatelessValidatorRethIo};

use witness_generator::StatelessValidationFixture;

use ere_io::Io;

/// Reads fixture JSON files from a directory.
pub fn read_fixtures_from_path(path: &Path) -> Result<Vec<StatelessValidationFixture>> {
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
pub fn generate_reth_input_from_fixture(
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

pub fn save_reth_input_to_file(
    reth_input: StatelessValidatorRethInput,
    file_name: &str,
    output_dir: &Path,
) -> Result<()> {
    let filename = sanitize_filename(file_name);

    std::fs::create_dir_all(output_dir)?;

    // Define output path with sanitized filename
    let output_path = output_dir.join(format!("{}.bin", filename));

    // Serialize the reth input to binary format
    let bytes = StatelessValidatorRethIo::serialize_input(&reth_input)
        .map_err(|e| anyhow::anyhow!("Failed to serialize reth input: {}", e))?;

    // Write the binary data to the output file
    std::fs::write(&output_path, bytes)
        .with_context(|| format!("Failed to write reth input to {}", output_path.display()))?;

    Ok(())
}

/// Generate an ethrex input from a fixture.
pub fn generate_ethrex_input_from_fixture(
    fixture: &StatelessValidationFixture,
) -> Result<StatelessValidatorEthrexInput> {
    let ethrex_input =
        StatelessValidatorEthrexInput::new(&fixture.stateless_input, fixture.success)
            .with_context(|| {
                format!(
                    "Failed to create StatelessValidatorEthrex input for {}",
                    fixture.name
                )
            })?;
    Ok(ethrex_input)
}

pub fn save_ethrex_input_to_file(
    ethrex_input: StatelessValidatorEthrexInput,
    file_name: &str,
    output_dir: &Path,
) -> Result<()> {
    let filename = sanitize_filename(file_name);

    std::fs::create_dir_all(output_dir)?;

    // Define output path with sanitized filename
    let output_path = output_dir.join(format!("{}.bin", filename));

    // Serialize the ethrex input to binary format
    let bytes = StatelessValidatorEthrexIo::serialize_input(&ethrex_input)
        .map_err(|e| anyhow::anyhow!("Failed to serialize Ethrex input: {}", e))?;

    // Write the binary data to the output file
    std::fs::write(&output_path, bytes)
        .with_context(|| format!("Failed to write ethrex input to {}", output_path.display()))?;

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

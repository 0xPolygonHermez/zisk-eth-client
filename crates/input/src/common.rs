use anyhow::{Context, Result};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

use stateless_validator_reth::guest::StatelessValidatorRethInput;
use witness_generator::StatelessValidationFixture;

use crate::types::OutputFormat;

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

/// Generate reth inputs from a list of fixtures.
pub fn generate_reth_inputs(
    fixtures: &[StatelessValidationFixture],
    output: &Path,
    format: OutputFormat,
) -> Result<()> {
    let mut success_count = 0;
    let mut error_count = 0;

    for fixture in fixtures {
        match generate_reth_input(fixture, output, format) {
            Ok(_) => {
                info!("Generated input for: {}", fixture.name);
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to generate input for {}: {}", fixture.name, e);
                error_count += 1;
            }
        }
    }

    info!(
        "Completed: {} succeeded, {} failed",
        success_count, error_count
    );

    Ok(())
}

/// Generate a reth input from a fixture.
pub fn generate_reth_input(
    fixture: &StatelessValidationFixture,
    output_dir: &Path,
    format: OutputFormat,
) -> Result<()> {
    let reth_input = StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
        .with_context(|| {
            format!(
                "Failed to create StatelessValidatorReth input for {}",
                fixture.name
            )
        })?;

    let filename = sanitize_filename(&fixture.name);

    match format {
        OutputFormat::Binary => {
            std::fs::create_dir_all(output_dir)?;
            let output_path = output_dir.join(format!("{}.bin", filename));
            let bytes = bincode::serialize(&reth_input)?;
            std::fs::write(&output_path, bytes)?;
        }
        OutputFormat::Json => {
            std::fs::create_dir_all(output_dir)?;
            let output_path = output_dir.join(format!("{}.json", filename));
            let json = serde_json::to_string_pretty(&reth_input)?;
            std::fs::write(&output_path, json)?;
        }
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

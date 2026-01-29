use anyhow::{Context, Result};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

use reth_stateless::StatelessInput;

use stateless_validator_reth::guest::StatelessValidatorRethInput;

use crate::OutputFormat;

// TODO: Import from witness-generator when fixed
/// A stateless validation fixture containing block data and witness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidationFixture {
    /// Name of the blockchain test case (e.g., "`ModExpAttackContract`").
    pub name: String,
    /// The stateless input for the block validation.
    pub stateless_input: StatelessInput,
    /// Whether the stateless block validation is successful.
    pub success: bool,
}

pub fn process_fixtures(input: &Path, output: &Path, format: OutputFormat) -> Result<()> {
    info!("Reading fixtures from: {}", input.display());

    let fixtures = read_benchmark_fixtures(input)?;
    info!("Found {} fixtures", fixtures.len());

    let mut success_count = 0;
    let mut error_count = 0;
    for fixture in &fixtures {
        match generate_reth_inputs_from_fixtures(fixture, output, format) {
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

/// Reads the benchmark fixtures folder and returns a list of block and witness pairs.
pub fn read_benchmark_fixtures(path: &Path) -> Result<Vec<StatelessValidationFixture>> {
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

pub fn generate_reth_inputs_from_fixtures(
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
            let bin_dir = output_dir.join("bin");
            std::fs::create_dir_all(&bin_dir)?;
            let output_path = bin_dir.join(format!("{}.bin", filename));
            let bytes = bincode::serialize(&reth_input)?;
            std::fs::write(&output_path, bytes)?;
        }
        OutputFormat::Json => {
            let json_dir = output_dir.join("json");
            std::fs::create_dir_all(&json_dir)?;
            let output_path = json_dir.join(format!("{}.json", filename));
            let json = serde_json::to_string_pretty(&reth_input)?;
            std::fs::write(&output_path, json)?;
        }
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

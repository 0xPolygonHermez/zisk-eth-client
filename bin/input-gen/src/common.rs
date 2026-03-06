use anyhow::{Context, Result};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::path::Path;
use walkdir::WalkDir;

use witness_generator::StatelessValidationFixture;

use zisk_sdk::{ZiskIO, ZiskStdin};

use guest::{RethInput, RethInputPublic, RethInputWitness};

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
pub fn reth_input_from_fixture(fixture: &StatelessValidationFixture) -> Result<RethInput> {
    let reth_input = RethInput::new(&fixture.stateless_input)
        .with_context(|| format!("Failed to create RethInput input for {}", fixture.name))?;
    Ok(reth_input)
}

pub fn reth_input_to_file(
    reth_input: RethInput,
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
    let output_path = output_dir.join(format!("{}.{}", filename, extension));

    let zisk_stdin = ZiskStdin::new();

    // Write public
    let public = RethInputPublic {
        public_keys: reth_input.public_keys,
    };
    let pk_bytes = match format {
        OutputFormat::Binary => bincode::serialize(&public)?,
        OutputFormat::Json => serde_json::to_vec_pretty(&public)?,
    };
    zisk_stdin.write_slice(&pk_bytes);

    // Write witness
    let witness = RethInputWitness {
        stateless_input: reth_input.stateless_input,
    };
    let main_bytes = match format {
        OutputFormat::Binary => bincode::serialize(&witness)?,
        OutputFormat::Json => serde_json::to_vec_pretty(&witness)?,
    };
    zisk_stdin.write_slice(&main_bytes);

    zisk_stdin.save(&output_path)?;

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

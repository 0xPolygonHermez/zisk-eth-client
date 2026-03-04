use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use witness_generator::{eest_generator::EESTFixtureGeneratorBuilder, FixtureGenerator};

use crate::{
    common::{fixtures_from_path, reth_input_from_fixture, reth_input_to_file},
    types::OutputFormat,
};

/// Process EEST (Ethereum Execution Specification Tests) to generate reth inputs.
pub async fn reth_input_files_from_tests(
    tag: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    eest_fixtures_path: Option<PathBuf>,
    output: &Path,
    format: OutputFormat,
    num_threads: Option<usize>,
) -> Result<()> {
    if let Some(threads) = num_threads {
        std::env::set_var("RAYON_NUM_THREADS", threads.to_string());
    }

    let mut builder = EESTFixtureGeneratorBuilder::default();

    if let Some(tag) = tag {
        info!("Using EEST release tag: {}", tag);
        builder = builder.with_tag(tag);
    } else if let Some(input_folder) = eest_fixtures_path {
        info!("Using local EEST fixtures from: {}", input_folder.display());
        builder = builder.with_input_folder(input_folder)?;
    } else {
        info!("Using latest EEST release");
    }

    if let Some(include) = include {
        info!("Include patterns: {:?}", include);
        builder = builder.with_includes(include);
    }
    if let Some(exclude) = exclude {
        info!("Exclude patterns: {:?}", exclude);
        builder = builder.with_excludes(exclude);
    }

    let generator = builder
        .build()
        .await
        .context("Failed to build EEST generator")?;

    info!("Generating EEST fixtures...");

    // Generate fixtures to a temp directory, then convert to reth inputs
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    let count = generator
        .generate_to_path(temp_dir.path())
        .await
        .context("Failed to generate EEST fixtures")?;

    info!(
        "Generated {} EEST fixtures, converting to reth inputs...",
        count
    );

    let fixtures = fixtures_from_path(temp_dir.path())?;

    let mut success_count = 0;
    let mut error_count = 0;
    for fixture in &fixtures {
        match reth_input_from_fixture(fixture) {
            Ok(reth_input) => {
                reth_input_to_file(reth_input, &fixture.name, output, format)?;
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

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;
use witness_generator::{eest_generator::EESTFixtureGeneratorBuilder, FixtureGenerator};

use crate::{
    common::{generate_reth_inputs, read_fixtures_from_path},
    OutputFormat,
};

/// Process EEST (Ethereum Execution Specification Tests) to generate reth inputs.
pub(crate) async fn process_tests(
    tag: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    eest_fixtures_path: Option<PathBuf>,
    output: &Path,
    format: OutputFormat,
    num_threads: Option<usize>,
) -> Result<()> {
    // Set RAYON_NUM_THREADS if specified (must be set before rayon initializes)
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

    let fixtures = read_fixtures_from_path(temp_dir.path())?;
    generate_reth_inputs(&fixtures, output, format)
}

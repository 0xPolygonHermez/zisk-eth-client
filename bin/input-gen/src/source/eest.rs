use anyhow::{Context, Result};
use rayon::ThreadPoolBuilder;
use std::path::{Path, PathBuf};
use tracing::info;
use witness_generator::{eest_generator::EESTFixtureGeneratorBuilder, FixtureGenerator};

use crate::{client::ExecutionClient, common::fixtures_from_path, processor::ProcessingTracker};

/// Process EEST (Ethereum Execution Specification Tests) to generate reth inputs.
pub async fn zisk_inputs_from_eest(
    tag: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    eest_fixtures_path: Option<PathBuf>,
    output: &Path,
    client: &dyn ExecutionClient,
    num_threads: Option<usize>,
) -> Result<()> {
    if let Some(threads) = num_threads {
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build global Rayon thread pool");
    }

    let mut builder = EESTFixtureGeneratorBuilder::default();

    if let Some(tag) = tag {
        info!("Using EEST release tag: {}", tag);
        builder = builder.with_tag(tag);
    } else if let Some(input_folder) = eest_fixtures_path {
        info!("Using local EEST from: {}", input_folder.display());
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

    // Generate fixtures to a temp directory, then convert to ZisK inputs
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    let count = generator
        .generate_to_path(temp_dir.path())
        .await
        .context("Failed to generate EEST fixtures")?;

    info!(
        "Generated {} EEST fixtures, converting to ZisK inputs...",
        count
    );

    // Initialize the tracker
    let mut tracker = ProcessingTracker::new(client.display_name());

    let fixtures = fixtures_from_path(temp_dir.path())?;
    for fixture in &fixtures {
        let name = format!("EEST \"{}\"", fixture.name);
        match client.process_fixture(fixture, output) {
            Ok(_) => tracker.record_success(&name),
            Err(e) => tracker.record_error(&name, &e),
        }
    }

    tracker.log_summary();

    Ok(())
}

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use witness_generator::{eest_generator::EESTFixtureGeneratorBuilder, FixtureGenerator};

use crate::{client::ExecutionClient, common::read_fixtures_from_path};

/// Process EEST (Ethereum Execution Specification Tests) to generate inputs
#[allow(clippy::too_many_arguments)]
pub async fn zisk_inputs_from_eest(
    tag: Option<String>,
    includes: Option<Vec<String>>,
    excludes: Option<Vec<String>>,
    eest_fixtures_path: Option<PathBuf>,
    output: &Path,
    num_threads: Option<usize>,
    client: &dyn ExecutionClient,
) -> Result<()> {
    if let Some(threads) = num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build global Rayon thread pool");
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

    if let Some(includes) = includes {
        info!("Include patterns: {:?}", includes);
        builder = builder.with_includes(includes);
    }
    if let Some(excludes) = excludes {
        info!("Exclude patterns: {:?}", excludes);
        builder = builder.with_excludes(excludes);
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

    let fixtures = read_fixtures_from_path(temp_dir.path())?;

    let mut success_count = 0;
    let mut error_count = 0;
    for fixture in &fixtures {
        match client.generate_input(fixture) {
            Ok(result) => {
                result.save_to_file(&fixture.name, output)?;
                info!("Generated {} input for: {}", client.name(), fixture.name);
                success_count += 1;
            }
            Err(e) => {
                warn!(
                    "Failed to generate {} input for {}: {}",
                    client.name(),
                    fixture.name,
                    e
                );
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

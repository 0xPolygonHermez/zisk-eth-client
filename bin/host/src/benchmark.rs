use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{error, info};

use zisk_sdk::GuestProgram;

use crate::{
    cli::Action,
    zisk::{ZiskClient, ZiskExecutionMetrics},
};

pub struct BenchmarkRunner {
    action: Action,
    output_folder: Option<PathBuf>,
    force_rerun: bool,
    zisk_client: ZiskClient,
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkResult {
    test_name: String,
    time: f64,
    metrics: ZiskExecutionMetrics,
}

impl BenchmarkRunner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        elf: GuestProgram,
        action: Action,
        output_folder: Option<PathBuf>,
        force_rerun: bool,
        proving_key: Option<PathBuf>,
        emulator: bool,
        unlock_mapped_memory: bool,
        gpu: bool,
    ) -> Result<Self> {
        // Execute and verify-constraints don't aggregate proofs.
        let no_aggregation = matches!(action, Action::Execute | Action::VerifyConstraints);
        let zisk_client = ZiskClient::new(elf).with_proving_key(
            proving_key,
            emulator,
            unlock_mapped_memory,
            gpu,
            no_aggregation,
        )?;

        Ok(Self {
            action,
            output_folder,
            force_rerun,
            zisk_client,
        })
    }

    pub async fn run(
        &self,
        input_folder: &Path,
        include: Option<&[String]>,
        exclude: Option<&[String]>,
    ) -> Result<()> {
        self.zisk_client.setup().await?;

        let mut input_files = collect_input_files(input_folder)?;

        if let Some(patterns) = include {
            info!("Include patterns: {:?}", patterns);
            input_files.retain(|file| {
                let name = file.to_string_lossy();
                patterns.iter().any(|p| name.contains(p))
            });
        }

        if let Some(patterns) = exclude {
            info!("Exclude patterns: {:?}", patterns);
            input_files.retain(|file| {
                let name = file.to_string_lossy();
                !patterns.iter().any(|p| name.contains(p))
            });
        }

        let total = input_files.len();
        info!("Found {} input files to run", total);

        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        for (index, file) in input_files.iter().enumerate() {
            match self.run_single(file, index + 1, total).await {
                Ok(true) => passed += 1,
                Ok(false) => skipped += 1,
                Err(e) => {
                    error!("Failed to run benchmark for {}: {}", file.display(), e);
                    failed += 1;
                }
            }
        }

        info!("");
        info!(
            "Summary: {} passed, {} failed, {} skipped",
            passed, failed, skipped
        );

        Ok(())
    }

    async fn run_single(&self, input_file: &Path, current: usize, total: usize) -> Result<bool> {
        let test_name = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        match &self.action {
            Action::Execute => {
                if let Some(ref output_folder) = self.output_folder {
                    let filename = input_file.file_name().unwrap_or_default();
                    let output_file = output_folder.join(filename).with_extension("json");

                    if output_file.exists() && !self.force_rerun {
                        info!("[{}/{}] Skipping {}", current, total, test_name);
                        return Ok(false);
                    }
                }

                info!("[{}/{}] Running: {}", current, total, test_name);

                let metrics = self.zisk_client.execute(input_file).await?;
                let elapsed = metrics.duration.as_secs_f64();

                info!("Execution metrics: {:?}", metrics);
                info!("[{}/{}] Completed in {:.2}s", current, total, elapsed);

                if let Some(ref output_folder) = self.output_folder {
                    let filename = input_file.file_name().unwrap_or_default();
                    let output_file = output_folder.join(filename).with_extension("json");

                    if let Some(parent) = output_file.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    let result = BenchmarkResult {
                        test_name: test_name.to_string(),
                        time: elapsed,
                        metrics,
                    };

                    let output_json = serde_json::to_string_pretty(&result)?;
                    fs::write(&output_file, output_json)?;
                }
            }

            Action::VerifyConstraints => {
                info!(
                    "[{}/{}] Verifying constraints: {}",
                    current, total, test_name
                );

                let metrics = self.zisk_client.verify_constraints(input_file).await?;
                let elapsed = metrics.duration.as_secs_f64();

                info!("[{}/{}] PASSED in {:.2}s", current, total, elapsed);
            }

            Action::Prove => {
                unimplemented!("Prove action is not implemented yet");
            }
        }

        Ok(true)
    }
}

fn collect_input_files(input_folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if input_folder.is_dir() {
        for entry in fs::read_dir(input_folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        files.push(sub_path);
                    }
                }
            } else if path.is_file() {
                files.push(path);
            }
        }
    } else {
        files.push(input_folder.to_path_buf());
    }

    files.sort();
    Ok(files)
}

use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tracing::{error, info};

use crate::cli::Cli;
use crate::zisk::{self, ExecutionMetrics};

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub time: f64,
    pub metrics: ExecutionMetrics,
}

pub struct BenchmarkRunner<'a> {
    cli: &'a Cli,
    output_folder: PathBuf,
}

impl<'a> BenchmarkRunner<'a> {
    pub fn new(cli: &'a Cli) -> Self {
        Self {
            cli,
            output_folder: cli.output_folder.clone(),
        }
    }

    pub fn run(&self, input_folder: &Path) -> Result<()> {
        let input_files = collect_input_files(input_folder)?;
        let total = input_files.len();
        info!("Found {} input files to run", total);

        for (index, file) in input_files.iter().enumerate() {
            if let Err(e) = self.run_single(file, index + 1, total) {
                error!("Failed to run benchmark for {}: {}", file.display(), e);
            }
        }

        Ok(())
    }

    fn run_single(&self, input_file: &Path, current: usize, total: usize) -> Result<()> {
        let test_name = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let filename = input_file.file_name().unwrap_or_default();
        let output_file = self.output_folder.join(filename).with_extension("json");

        if output_file.exists() && !self.cli.force_rerun {
            info!("[{}/{}] Skipping {}", current, total, test_name);
            return Ok(());
        }

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)?;
        }

        info!("[{}/{}] Running: {}", current, total, test_name);

        let start = Instant::now();
        let metrics = zisk::execute(&self.cli.ziskemu, &self.cli.elf, input_file)?;
        let elapsed = start.elapsed();

        info!(
            "[{}/{}] Completed in {:.2}s",
            current,
            total,
            elapsed.as_secs_f64(),
        );

        let result = BenchmarkResult {
            test_name: test_name.to_string(),
            time: elapsed.as_secs_f64(),
            metrics,
        };

        let output_json = serde_json::to_string_pretty(&result)?;
        fs::write(&output_file, output_json)?;

        Ok(())
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

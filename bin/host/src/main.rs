use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod cli;
use cli::{Action, Cli, Client, GuestProgramCommand};

// TODO: zisk commands (ziskemu, cargo-zisk) should be called via a library instead of spawning processes.

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match &cli.action {
        Action::Execute => { /* proceed */ }
        Action::VerifyConstraints => {
            info!("Verifying constraints is not yet implemented");
            return Ok(());
        }
        Action::Prove => {
            info!("Generating proofs is not yet implemented");
            return Ok(());
        }
    }

    info!("ZisK Ethereum Client Host");
    info!(" Guest Program: {}", cli.guest_program.display_name());
    info!(" Action: {:?}", cli.action);
    info!(" ELF: {}\n", cli.elf.display());

    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            client,
        } => {
            run_stateless_validator(&cli, input_folder, *client)?;
        }
    }

    Ok(())
}

fn run_stateless_validator(cli: &Cli, input_folder: &Path, client: Client) -> Result<()> {
    // Collect test files
    let input_files = collect_input_files(input_folder)?;
    let total = input_files.len();
    info!("Found {} input files to run", total);

    // Create output folder with client subfolder
    let output_folder = cli
        .output_folder
        .join(format!("stateless-validator-{:?}", client).to_lowercase());

    // Run benchmarks
    for (index, file) in input_files.iter().enumerate() {
        if let Err(e) = run_benchmark(cli, &output_folder, input_folder, file, index + 1, total) {
            error!("Failed to run benchmark for {}: {}", file.display(), e);
        }
    }

    Ok(())
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
        // Single file
        files.push(input_folder.to_path_buf());
    }

    files.sort();
    Ok(files)
}

fn run_benchmark(
    cli: &Cli,
    output_folder: &Path,
    input_folder: &Path,
    input_file: &Path,
    current: usize,
    total: usize,
) -> Result<()> {
    let test_name = input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Determine output file path
    let output_file = if input_folder.is_dir() {
        // Preserve folder structure
        let relative_path = input_file.strip_prefix(input_folder).unwrap_or(input_file);
        output_folder.join(relative_path).with_extension("json")
    } else {
        // Single file input
        let filename = input_file.file_name().unwrap_or_default();
        output_folder.join(filename).with_extension("json")
    };

    // Skip if output exists and not forcing
    if output_file.exists() && !cli.force_rerun {
        info!("[{}/{}] Skipping {}", current, total, test_name);
        return Ok(());
    }

    // Create output directory
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent)?;
    }

    info!("[{}/{}] Running: {}", current, total, test_name);

    let start = Instant::now();

    // Run ZisK with the input file directly
    let result = run_zisk(&cli.ziskemu, &cli.elf, input_file);

    let elapsed = start.elapsed();

    match result {
        Ok(metrics) => {
            info!(
                "[{}/{}] Completed in {:.2}s",
                current,
                total,
                elapsed.as_secs_f64(),
            );

            let output = BenchmarkResult {
                test_name: test_name.to_string(),
                action: cli.action.clone(),
                time: elapsed.as_secs_f64(),
                metrics,
            };
            let output_json = serde_json::to_string_pretty(&output)?;
            fs::write(&output_file, output_json)?;
        }
        Err(e) => {
            error!("[{}/{}] Failed: {}", current, total, e);
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ExecutionMetrics {
    steps: u64,
    cost: u64,
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkResult {
    test_name: String,
    action: Action,
    time: f64,
    metrics: ExecutionMetrics,
}

fn run_zisk(ziskemu: &Path, elf: &Path, input_file: &Path) -> Result<ExecutionMetrics> {
    let mut cmd = Command::new(ziskemu);
    cmd.arg("-e")
        .arg(elf)
        .arg("-i")
        .arg(input_file)
        .arg("--stats");

    let output = cmd.output().context("Failed to run ziskemu")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ziskemu failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_zisk_metrics(&stdout)
}

fn parse_zisk_metrics(output: &str) -> Result<ExecutionMetrics> {
    let mut steps = 0u64;
    let mut cost = 0u64;

    for line in output.lines() {
        if line.contains("STEPS")
            && let Some(val) = line.split_whitespace().last()
        {
            steps = val.replace(",", "").parse().unwrap_or(0);
        }
        if line.contains("TOTAL")
            && line.contains("100.00%")
            && let Some(val) = line.split_whitespace().nth(1)
        {
            cost = val.replace(",", "").parse().unwrap_or(0);
        }
    }

    Ok(ExecutionMetrics { steps, cost })
}

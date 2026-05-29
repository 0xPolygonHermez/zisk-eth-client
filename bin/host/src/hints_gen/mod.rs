use anyhow::{Context, Result};
use clap::Args;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, warn};
use zisk_sdk::ZiskStdin;

use input::{Client, ExecutionClient, create_client, generate_hints_to_file};

#[derive(Args, Debug, Clone)]
pub struct HintsGenArgs {
    /// Input .bin file(s)
    #[arg(conflicts_with = "inputs_folder")]
    inputs: Vec<PathBuf>,

    /// Directory containing .bin input files (processed in sorted order)
    #[arg(short = 'f', long)]
    inputs_folder: Option<PathBuf>,

    /// Output directory for hints files (default: same directory as input)
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Execution client
    #[arg(short, long, value_enum, default_value = "reth")]
    client: Client,
}

pub fn run(args: HintsGenArgs) -> Result<()> {
    if let Some(dir) = &args.output_dir {
        std::fs::create_dir_all(dir).context("Failed to create output directory")?;
    }

    let client = create_client(args.client);

    let inputs: Vec<PathBuf> = if let Some(folder) = &args.inputs_folder {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)
            .with_context(|| format!("Failed to read directory {}", folder.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort();
        entries
    } else if !args.inputs.is_empty() {
        args.inputs.clone()
    } else {
        anyhow::bail!("Provide at least one input file or use --inputs-folder");
    };

    let mut failed: Vec<(&Path, anyhow::Error)> = Vec::new();
    let mut timings: Vec<(Duration, Duration)> = Vec::new();
    for input in &inputs {
        match process_input_file(input, args.output_dir.as_deref(), client.as_ref()) {
            Ok(t) => timings.push(t),
            Err(e) => {
                error!("Failed {}: {:#}", input.display(), e);
                failed.push((input.as_path(), e));
            }
        }
    }

    if timings.len() > 1 {
        let n = timings.len() as u32;
        let avg_execution = timings.iter().map(|(e, _)| *e).sum::<Duration>() / n;
        let avg_total = timings.iter().map(|(_, t)| *t).sum::<Duration>() / n;
        info!(
            "Average over {} blocks — execution: {:.2?}, total: {:.2?}",
            n, avg_execution, avg_total,
        );
    }

    if !failed.is_empty() {
        warn!("{}/{} blocks failed:", failed.len(), inputs.len());
        for (path, _) in &failed {
            warn!("  {}", path.display());
        }
        anyhow::bail!("{} block(s) failed hints generation", failed.len());
    }

    Ok(())
}

fn process_input_file(
    input: &Path,
    output_dir: Option<&Path>,
    client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    let stdin = ZiskStdin::from_file(input).context("Failed to load input file")?;
    let output_path = match output_dir {
        Some(dir) => {
            let stem = input
                .file_stem()
                .with_context(|| format!("Input path has no file stem: {}", input.display()))?;
            dir.join(stem).with_extension("hints")
        }
        None => input.with_extension("hints"),
    };
    info!(
        "Generating hints: {} -> {}",
        input.display(),
        output_path.display()
    );
    generate_hints_to_file(&stdin, output_path, client)
}

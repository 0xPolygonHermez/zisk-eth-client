use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, warn};
use zisk_sdk::ZiskStdin;

use input::{create_client, Client, ExecutionClient};

#[derive(Parser, Debug)]
#[command(name = "hints-gen")]
#[command(about = "Generate ZisK hints from Ethereum block input files")]
#[command(
    long_about = "Runs block validation natively and captures hints for the ZisK prover.\n\nMust be compiled with: RUSTFLAGS=\"--cfg zisk_hints\" cargo build -p hints-gen"
)]
struct Cli {
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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    if let Some(dir) = &cli.output_dir {
        std::fs::create_dir_all(dir).context("Failed to create output directory")?;
    }

    let client = create_client(cli.client);

    let inputs: Vec<PathBuf> = if let Some(folder) = &cli.inputs_folder {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)
            .with_context(|| format!("Failed to read directory {}", folder.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort();
        entries
    } else if !cli.inputs.is_empty() {
        cli.inputs.clone()
    } else {
        anyhow::bail!("Provide at least one input file or use --inputs-folder");
    };

    let mut failed: Vec<(&Path, anyhow::Error)> = Vec::new();
    let mut timings: Vec<(Duration, Duration)> = Vec::new();
    for input in &inputs {
        match generate_hints_for_file(input, cli.output_dir.as_deref(), client.as_ref()) {
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

fn generate_hints_for_file(
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
    generate_hints_inner(stdin, output_path, client)
}

#[cfg(zisk_hints)]
fn generate_hints_inner(
    stdin: ZiskStdin,
    output_path: PathBuf,
    client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    use ziskos::hints::{close_hints, init_hints_file};

    ziskos::set_native_input(stdin.read_data());
    init_hints_file(output_path.clone(), None).context("Failed to init hints")?;

    let t0 = std::time::Instant::now();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.run()));
    let execution = t0.elapsed();

    let _ = close_hints();

    match run_result {
        Ok(()) => {
            let total = t0.elapsed();
            info!(
                "Written hints to {} (execution: {:.2?}, total: {:.2?})",
                output_path.display(),
                execution,
                total,
            );
            Ok((execution, total))
        }
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            Err(anyhow::anyhow!("Block execution failed: {}", msg))
        }
    }
}

#[cfg(not(zisk_hints))]
fn generate_hints_inner(
    _stdin: ZiskStdin,
    _output_path: PathBuf,
    _client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    anyhow::bail!(
        "Compiled without hints support. Rebuild with:\n  RUSTFLAGS=\"--cfg zisk_hints\" cargo build -p hints-gen"
    )
}

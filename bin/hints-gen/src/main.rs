use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use tracing::info;
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
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

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

    for input in &cli.inputs {
        generate_hints_for_file(input, cli.output_dir.as_deref(), client.as_ref())?;
    }

    Ok(())
}

fn generate_hints_for_file(
    input: &Path,
    output_dir: Option<&Path>,
    client: &dyn ExecutionClient,
) -> Result<()> {
    let stdin = ZiskStdin::from_file(input).context("Failed to load input file")?;
    let output_path = match output_dir {
        Some(dir) => dir
            .join(input.file_stem().unwrap_or_default())
            .with_extension("hints"),
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
) -> Result<()> {
    use ziskos::hints::{close_hints, init_hints_file};

    ziskos::set_native_input(stdin.read_raw_bytes());
    init_hints_file(output_path.clone()).context("Failed to init hints")?;

    let t0 = std::time::Instant::now();
    client.run();
    let execution = t0.elapsed();

    close_hints().context("Failed to write hints")?;
    let total = t0.elapsed();

    info!(
        "Written hints to {} (execution: {:.2?}, total: {:.2?})",
        output_path.display(),
        execution,
        total,
    );

    Ok(())
}

#[cfg(not(zisk_hints))]
fn generate_hints_inner(
    _stdin: ZiskStdin,
    _output_path: PathBuf,
    _client: &dyn ExecutionClient,
) -> Result<()> {
    anyhow::bail!(
        "Compiled without hints support. Rebuild with:\n  RUSTFLAGS=\"--cfg zisk_hints\" cargo build -p hints-gen"
    )
}

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use host::hints_gen::{run, HintsGenArgs};

#[derive(Parser, Debug)]
#[command(name = "hints-gen")]
#[command(
    about = "Generate ZisK hints from Ethereum block input files",
    long_about = "Runs block validation natively and captures hints for the ZisK prover.\n\nMust be compiled with: RUSTFLAGS=\"--cfg zisk_hints\" cargo build -p hints-gen"
)]
struct Cli {
    #[command(flatten)]
    args: HintsGenArgs,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    run(cli.args)
}

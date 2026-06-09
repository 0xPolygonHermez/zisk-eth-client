use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use host::input_gen::{run, InputGenArgs};

#[derive(Parser, Debug)]
#[command(version, about = "Generate ZisK inputs from RPC or EEST fixtures", long_about = None)]
struct Cli {
    #[command(flatten)]
    args: InputGenArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    run(cli.args).await
}

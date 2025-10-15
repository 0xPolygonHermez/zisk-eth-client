use clap::Parser;
use std::{path::PathBuf};
use tracing_subscriber::{
    filter::EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

mod types;
mod rsp;
mod zeth;
use types::{GuestProgram, InputGenerator, Network};
use rsp::RspInputGenerator;
use zeth::ZethInputGenerator;

use crate::types::InputGeneratorConfig;

#[derive(Debug, Clone, Parser)]
pub struct InputGenArgs {
    #[clap(long, short)]
    pub block_number: u64,

    #[clap(long, short, value_enum)]
    pub network: Option<Network>,

    #[clap(long, short)]
    pub rpc_url: String,

    #[clap(long, short)]
    pub guest: GuestProgram,

    #[clap(long, short)]
    pub input_dir: Option<PathBuf>,
}

pub fn make_input_generator(args: &InputGenArgs) -> Box<dyn InputGenerator> {
    let config = InputGeneratorConfig {
        guest: args.guest.clone(),
        rpc_url: Url::parse(&args.rpc_url).expect("Invalid RPC URL"),
        network: args.network.clone(),
        input_dir: args.input_dir.clone(),
    };

    match args.guest {
        GuestProgram::Rsp => Box::new(RspInputGenerator::new(config)),
        GuestProgram::Zeth => Box::new(ZethInputGenerator::new(config)),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the environment variables.
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Initialize the logger.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // Parse the command line arguments.
    let args = InputGenArgs::parse();
    let input_generator = make_input_generator( &args);

    println!("Generating input file fo block {}, guest: {}", args.block_number, args.guest);

    let start_time = std::time::Instant::now();
    let result = input_generator.generate(args.block_number).await?;

    println!(
        "Input file for block {} ({} txs, {} mgas) saved to {}, time: {} ms",
        args.block_number,
        result.tx_count,
        (result.gas_used + 999_999) / 1_000_000,
        result.input_file_path.to_string_lossy(),
        start_time.elapsed().as_millis()
    );

    Ok(())
}

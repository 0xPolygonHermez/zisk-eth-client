use clap::Parser;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_provider::create_provider;
use rsp_rpc_db::RpcDb;
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::{
    filter::EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

#[derive(Debug, Clone, Parser)]
pub struct InputGenArgs {
    #[clap(long, short)]
    pub block_number: u64,

    #[clap(long, short)]
    pub rpc_url: String,

    #[clap(long, short)]
    pub input_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
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

    println!("Generating input file fo block {}", args.block_number);

    // Create the RPC provider and database.
    let provider = create_provider(
        Url::parse(args.rpc_url.as_str())
            .expect("Invalid RPC URL"),
    );
    let rpc_db = RpcDb::new(provider.clone(), args.block_number - 1);
    let genesis = Genesis::Mainnet;

    let executor = EthHostExecutor::eth(
        Arc::new(
            (&genesis).try_into().expect("Failed to convert genesis block into the required type"),
        ),
        None,
    );
    
    let start_time = std::time::Instant::now();

    let input = executor
        .execute(args.block_number, &rpc_db, &provider, genesis.clone(), None, false)
        .await
        .expect("Failed to execute client");

    // Create the input directory if it does not exist.
    let input_folder = args.input_dir.unwrap_or("inputs".into());
    if !input_folder.exists() {
        std::fs::create_dir_all(&input_folder)?;
    }

    // Save the input to a file
    let mgas = input.current_block.header.gas_used / 1_000_000;
    let input_path = input_folder.join(format!(
        "{}_{}_{}.bin",
        args.block_number,
        input.current_block.body.transactions.len(),
        mgas
    ));
    let mut cache_file = std::fs::File::create(&input_path)?;

    bincode::serialize_into(&mut cache_file, &input)?;

    println!(
        "Input file for block {} ({} txs, {} mgas) saved to {}, time: {} ms",
        args.block_number,
        input.current_block.body.transactions.len(),
        mgas,
        input_path.to_string_lossy(),
        start_time.elapsed().as_millis()
    );

    Ok(())
}

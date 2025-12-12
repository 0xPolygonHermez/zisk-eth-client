use clap::Parser;
use input::{build_input_generator, GuestProgram, Network};
use std::{io::Write, path::PathBuf};

#[derive(Debug, Clone, Parser)]
pub struct InputGenArgs {
    #[clap(long, short)]
    pub block_number: u64,

    #[clap(long, short, value_enum, default_value_t = Network::Mainnet)]
    pub network: Network,

    #[clap(long, short)]
    pub rpc_url: String,

    #[clap(long, short, value_enum, default_value_t = GuestProgram::Rsp)]
    pub guest: GuestProgram,

    #[clap(long, short)]
    pub input_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the environment variables.
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }

    // Parse the command line arguments.
    let args = InputGenArgs::parse();
    let input_generator =
        build_input_generator(args.guest.clone(), &args.rpc_url, args.network.clone());

    println!(
        "Generating input file for block {}, guest: {}",
        args.block_number, args.guest
    );

    let start_time = std::time::Instant::now();
    let result = input_generator.generate(args.block_number).await?;

    // Create the input directory if it does not exist.
    let default_input_folder = match args.guest {
        GuestProgram::Rsp => PathBuf::from("bin/client/rsp/inputs"),
        GuestProgram::Zeth => PathBuf::from("bin/client/zeth/inputs"),
    };
    let input_folder = args.input_dir.clone().unwrap_or(default_input_folder);
    if !input_folder.exists() {
        std::fs::create_dir_all(&input_folder)?;
    }

    let mgas = result.gas_used / 1_000_000;

    let input_path = input_folder.join(format!(
        "{}_{}_{}_{}.bin",
        args.block_number, result.tx_count, mgas, args.guest
    ));

    let mut input_file = std::fs::File::create(&input_path)?;
    input_file.write_all(&result.input)?;

    println!(
        "Input file for block {} ({} txs, {} mgas) saved to {}, time: {} ms",
        args.block_number,
        result.tx_count,
        mgas,
        input_path.to_string_lossy(),
        start_time.elapsed().as_millis()
    );

    Ok(())
}

use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Write};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod benchmark;
mod cli;
mod zisk;

use benchmark::BenchmarkRunner;
use cli::{Cli, GuestProgramCommand};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    cli.validate().map_err(|e| anyhow::anyhow!(e))?;

    // Write metadata to a separate file
    write_run_metadata(&cli)?;

    info!("ZisK Ethereum Client Host");
    info!(" Guest Program: {}", cli.guest_program.display_name());
    info!(" Action: {:?}", cli.action);
    info!(" ELF: {}\n", cli.elf.display());

    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            client: _,
        } => {
            let runner = BenchmarkRunner::new(&cli);
            runner.run(input_folder)?;
        }
    }

    Ok(())
}

fn write_run_metadata(cli: &Cli) -> Result<()> {
    let log_path = cli.output_folder.join("run_metadata.log");

    // Create parent directory if needed
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&log_path)?;

    writeln!(file, "ZisK Ethereum Client Host")?;
    writeln!(file, "=========================")?;
    writeln!(file, "Guest Program: {}", cli.guest_program.display_name())?;
    writeln!(file, "Action: {:?}", cli.action)?;
    writeln!(file, "ELF: {}", cli.elf.display())?;

    // Add per-guest metadata
    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            client,
        } => {
            writeln!(file, "Input Folder: {}", input_folder.display())?;
            writeln!(file, "Client: {:?}", client)?;
        }
    }

    Ok(())
}

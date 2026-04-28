use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Write};
use tracing::info;
use zisk_sdk::VerboseMode;

mod benchmark;
mod cli;
mod elfs;
mod zisk;

use benchmark::BenchmarkRunner;
use cli::{Cli, Client, GuestProgramCommand};
use elfs::{ELF_ETHREX, ELF_RETH};

fn main() -> Result<()> {
    zisk_sdk::setup_logger(VerboseMode::Info);

    let cli = Cli::parse();

    // Write metadata to a separate file
    if cli.output_folder.is_some() {
        write_metadata(&cli)?;
    }

    info!("ZisK Host");
    if let Some(proving_key) = &cli.proving_key
        && let Some(name) = proving_key.file_name()
    {
        info!(" Proving Key: {}", name.to_string_lossy());
    }
    info!(" Action: {:?}", cli.action);
    info!(" Guest Program: {}", cli.guest_program.display_name());
    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            client,
            include,
            exclude,
        } => {
            info!(" Client: {:?}", client);

            let elf = match client {
                Client::Reth => ELF_RETH,
                Client::Ethrex => ELF_ETHREX,
            };

            info!(" ELF: {}", elf.name());
            info!(" Input Folder: {}", input_folder.display());
            if let Some(include) = include {
                info!(" Include Patterns: {:?}", include);
            }
            if let Some(exclude) = exclude {
                info!(" Exclude Patterns: {:?}", exclude);
            }

            // Create benchmark runner and execute benchmarks
            let runner = BenchmarkRunner::new(
                elf,
                cli.action,
                cli.output_folder.clone(),
                cli.force_rerun,
                cli.proving_key.clone(),
                cli.emulator,
                cli.port,
                cli.unlock_mapped_memory,
            )?;
            runner.run(input_folder, include.as_deref(), exclude.as_deref())?;
        }
    }

    Ok(())
}

fn write_metadata(cli: &Cli) -> Result<()> {
    let output_folder = cli.output_folder.as_ref().unwrap();
    let log_path = output_folder.join("metadata.log");

    // Create parent directory if needed
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&log_path)?;

    writeln!(file, "ZisK Host")?;
    writeln!(file, "=========================")?;
    writeln!(file, "Action: {:?}", cli.action)?;
    writeln!(file, "Guest Program: {}", cli.guest_program.display_name())?;

    // Add per-guest metadata
    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            client,
            include,
            exclude,
        } => {
            writeln!(file, "Client: {:?}", client)?;
            writeln!(file, "Input Folder: {}", input_folder.display())?;
            if let Some(include) = include {
                writeln!(file, "Include Patterns: {:?}", include)?;
            }
            if let Some(exclude) = exclude {
                writeln!(file, "Exclude Patterns: {:?}", exclude)?;
            }
        }
    }

    Ok(())
}

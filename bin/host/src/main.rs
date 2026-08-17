use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Write};
use tracing::info;
use zisk_sdk::VerboseMode;

use host::benchmark::BenchmarkRunner;
use host::cli::{Cli, GuestProgramCommand};
#[cfg(feature = "ziskethone")]
use host::elfs::ELF_ZISKETHONE;
use host::elfs::{ELF_ETHREX, ELF_RETH};
use input::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Write metadata to a separate file
    if cli.output_folder.is_some() {
        write_metadata(&cli)?;
    }

    info!("ZisK Host");
    if let Some(proving_key) = &cli.proving_key {
        info!(" Proving Key: {}", proving_key.display());
    }
    info!(" Action: {:?}", cli.action);
    info!(" Guest Program: {}", cli.guest_program.display_name());

    match &cli.guest_program {
        GuestProgramCommand::StatelessValidator {
            input_folder,
            hints,
            gen_hints,
            hints_out,
            client,
            include,
            exclude,
        } => {
            info!(" Client: {:?}", client);

            // To add a client, see docs/adding-a-client.md.
            let elf = match client {
                Client::Reth => ELF_RETH,
                Client::Ethrex => ELF_ETHREX,
                #[cfg(feature = "ziskethone")]
                Client::ZiskEthOne => ELF_ZISKETHONE,
            };

            info!(" ELF Name: {}", elf.name());
            if let Some(input_folder) = input_folder {
                info!(" Input Folder: {}", input_folder.display());
            }
            if let Some(hints) = hints {
                info!(" Hints: {}", hints.display());
            }
            if *gen_hints {
                info!(" Generating hints before running");
            }
            if let Some(include) = include {
                info!(" Include Patterns: {:?}", include);
            }
            if let Some(exclude) = exclude {
                info!(" Exclude Patterns: {:?}", exclude);
            }

            let runner = BenchmarkRunner::new(
                elf,
                cli.action.clone(),
                cli.output_folder.clone(),
                cli.force_rerun,
                cli.proving_key.clone(),
                cli.emulator,
                cli.unlock_mapped_memory,
                cli.gpu,
                hints.clone(),
                *gen_hints,
                hints_out.clone(),
                *client,
            )?;
            runner
                .run(
                    input_folder.as_deref(),
                    include.as_deref(),
                    exclude.as_deref(),
                )
                .await?;
        }
    }

    Ok(())
}

fn init_tracing(verbose: u8) {
    let mode = match verbose {
        0 => VerboseMode::Info,
        1 => VerboseMode::Debug,
        _ => VerboseMode::Trace,
    };
    zisk_sdk::setup_logger(mode);
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
            hints,
            gen_hints,
            hints_out,
            client,
            include,
            exclude,
        } => {
            writeln!(file, "Client: {:?}", client)?;
            if let Some(input_folder) = input_folder {
                writeln!(file, "Input Folder: {}", input_folder.display())?;
            }
            if let Some(hints) = hints {
                writeln!(file, "Hints: {}", hints.display())?;
            }
            if *gen_hints {
                writeln!(file, "Generate Hints: true")?;
            }
            if let Some(hints_out) = hints_out {
                writeln!(file, "Hints Out: {}", hints_out.display())?;
            }
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

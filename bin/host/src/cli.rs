use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// ZisK Ethereum Client Host - Benchmark runner
#[derive(Parser, Debug)]
#[command(name = "zec-host")]
#[command(about = "Run ZisK Ethereum Client benchmarks")]
#[command(version)]
pub struct Cli {
    /// Action to perform
    #[arg(short, long, value_enum, default_value = "execute")]
    pub action: Action,

    /// Force rerun even if results exist
    #[arg(long, default_value_t = false)]
    pub force_rerun: bool,

    /// Guest program to benchmark
    #[command(subcommand)]
    pub guest_program: GuestProgramCommand,

    /// Output folder for benchmark results
    #[arg(short, long, default_value = "metrics")]
    pub output_folder: PathBuf,

    /// Path to the compiled guest program ELF binary
    #[arg(long)]
    pub elf: PathBuf,

    /// Path to ziskemu binary
    #[arg(long, default_value = "ziskemu")]
    pub ziskemu: PathBuf,
}

/// Actions to perform
#[derive(Debug, Clone, ValueEnum, serde::Serialize)]
pub enum Action {
    /// Execute
    Execute,
    /// Verify constraints
    VerifyConstraints,
    /// Generate proof
    Prove,
}

/// Subcommands for different guest programs
#[derive(Subcommand, Clone, Debug)]
pub enum GuestProgramCommand {
    /// Ethereum Stateless Validator
    StatelessValidator {
        /// Input folder
        #[arg(short, long)]
        input_folder: PathBuf,
        /// Client
        #[arg(short, long, default_value = "reth")]
        client: Client,
    },
    // Add more guest programs here as needed
}

impl GuestProgramCommand {
    /// Returns the display name including client if applicable
    pub fn display_name(&self) -> String {
        match self {
            Self::StatelessValidator { client, .. } => {
                format!("Stateless Validator ({:?})", client)
            }
        }
    }
}

/// Execution clients for the stateless validator
#[derive(Debug, Copy, Clone, ValueEnum, serde::Serialize)]
pub enum Client {
    Reth,
    //Add more execution clients here as needed
}

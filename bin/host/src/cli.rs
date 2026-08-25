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
    #[arg(short, long, default_value_t = false)]
    pub force_rerun: bool,

    /// Guest program to benchmark
    #[command(subcommand)]
    pub guest_program: GuestProgramCommand,

    /// Output folder for benchmark results
    #[arg(short, long)]
    pub output_folder: Option<PathBuf>,

    /// Path to the proving key file (default: installed one)
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Use emulator backend (Emu) instead of assembly (Asm)
    #[arg(short = 'l', long, default_value_t = false)]
    pub emulator: bool,

    #[arg(long, conflicts_with = "emulator", default_value_t = false)]
    pub unlock_mapped_memory: bool,

    /// Use GPU acceleration
    #[arg(long, default_value_t = false)]
    pub gpu: bool,

    /// Report proving cost alongside the execution metrics.
    #[arg(long, default_value_t = false)]
    pub with_cost: bool,

    /// Increase log verbosity (`-v` = debug, `-vv` = trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
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
        /// Input folder or file (mutually exclusive with --hints)
        #[arg(
            short,
            long,
            conflicts_with = "hints",
            required_unless_present = "hints"
        )]
        input_folder: Option<PathBuf>,

        #[arg(long)]
        hints: Option<PathBuf>,

        #[arg(long, requires = "input_folder", conflicts_with = "hints")]
        gen_hints: bool,

        #[arg(long, requires = "gen_hints")]
        hints_out: Option<PathBuf>,

        /// Client
        #[arg(short, long, default_value = "reth")]
        client: input::Client,

        /// Include only files containing the provided strings.
        #[arg(long)]
        include: Option<Vec<String>>,

        /// Exclude files containing the provided strings.
        #[arg(long)]
        exclude: Option<Vec<String>>,
    },
    // Add more guest programs here as needed
}

impl GuestProgramCommand {
    pub fn display_name(&self) -> String {
        match self {
            Self::StatelessValidator { .. } => "Stateless Validator".to_string(),
        }
    }
}

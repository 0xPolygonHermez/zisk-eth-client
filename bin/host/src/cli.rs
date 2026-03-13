use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing::{error, warn};

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

    /// Path to the compiled guest program ELF binary
    #[arg(short, long)]
    pub elf: PathBuf,

    /// Path to the proving key file (default: installed one)
    #[arg(short, long)]
    pub proving_key: Option<PathBuf>,

    /// Path to ziskemu binary
    #[arg(long)]
    pub ziskemu: Option<PathBuf>,

    /// Use emulator backend (Emu) instead of assembly (Asm)
    #[arg(short = 'l', long, default_value_t = false)]
    pub emulator: bool,
}

impl Cli {
    pub fn validate(&self) -> Result<(), String> {
        match self.action {
            Action::VerifyConstraints | Action::Prove | Action::Execute => {
                if self.ziskemu.is_some() {
                    warn!(
                        "ZisK Emulator path is ignored when action is {:?}",
                        self.action
                    );
                }
            }
            Action::Ziskemu => {
                if self.proving_key.is_some() {
                    warn!("Proving key is ignored when action is {:?}", self.action);
                }

                if self.ziskemu.is_none() {
                    error!(
                        "ZisK Emulator path is required for action {:?}",
                        self.action
                    );
                    return Err(format!(
                        "ZisK Emulator path is required for action {:?}",
                        self.action
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Actions to perform
#[derive(Debug, Clone, ValueEnum, serde::Serialize)]
pub enum Action {
    /// Execute
    Execute,
    /// Ziskemu
    Ziskemu,
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

        /// Filter tests by gas value in millions (e.g., 1, 5, 10, 20, 30, 60).
        /// Only tests with "gas-value_XM" matching this value will be run.
        #[arg(short = 'g', long)]
        gas_millions: Option<u32>,
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

/// Execution clients for the stateless validator
#[derive(Debug, Copy, Clone, ValueEnum, serde::Serialize)]
pub enum Client {
    Reth,
    //Add more execution clients here as needed
}

use anyhow::{Context, Ok, Result};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use zisk_sdk::{Asm, ElfBinary, Emu, ProverClient, ZiskProgramPK, ZiskProver, ZiskStdin};

/// ZisK client
pub struct ZiskClient {
    pub elf: ElfBinary,
    pub backend: Option<ZiskBackend>,
    pub pk: Option<ZiskProgramPK>,
}

/// ZisK backend wrapper
pub enum ZiskBackend {
    Emu(ZiskProver<Emu>),
    Asm(ZiskProver<Asm>),
}

/// Output metrics from ZisK execution
#[derive(Debug, serde::Serialize)]
pub struct ZiskExecutionMetrics {
    #[serde(skip)]
    pub duration: Duration,
    pub steps: u64,
    pub cost: u64,
    pub tx_count: Option<u64>,
    pub gas_used: Option<u64>,
}

impl ZiskClient {
    pub fn new(elf: ElfBinary) -> Self {
        Self {
            elf,
            backend: None,
            pk: None,
        }
    }

    pub fn with_proving_key(
        mut self,
        proving_key: Option<PathBuf>,
        use_emulator: bool,
        port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let client = if use_emulator {
            let prover = ProverClient::builder()
                .emu()
                .verify_constraints()
                .proving_key_path_opt(proving_key)
                .build()
                .context("Failed to build ProverClient builder")?;

            let (pk, _) = prover.setup(&self.elf).context("Failed to setup program")?;
            self.pk = Some(pk);
            ZiskBackend::Emu(prover)
        } else {
            let prover = ProverClient::builder()
                .asm()
                .verify_constraints()
                .proving_key_path_opt(proving_key)
                .base_port_opt(port)
                .unlock_mapped_memory(unlock_mapped_memory)
                .build()
                .context("Failed to build ProverClient builder")?;

            let (pk, _) = prover.setup(&self.elf).context("Failed to setup program")?;
            self.pk = Some(pk);
            ZiskBackend::Asm(prover)
        };

        self.backend = Some(client);

        Ok(self)
    }

    /// Execute the program with the given input and return execution metrics
    pub fn execute(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let pk = self
            .pk
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proving key is not set up"))?;

        let client = self
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        let result = match client {
            ZiskBackend::Emu(prover) => prover.execute(pk, stdin)?,
            ZiskBackend::Asm(prover) => prover.execute(pk, stdin)?,
        };

        Ok(ZiskExecutionMetrics {
            duration: result.get_duration(),
            steps: result.get_execution_steps(),
            cost: result.get_execution_total_cost(),
            tx_count: None,
            gas_used: None,
        })
    }

    /// Verify constraints for the given input file
    pub fn verify_constraints(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let pk = self
            .pk
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proving key is not set up"))?;

        let client = self
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        let result = match client {
            ZiskBackend::Emu(prover) => prover.verify_constraints(pk, stdin)?,
            ZiskBackend::Asm(prover) => prover.verify_constraints(pk, stdin)?,
        };

        Ok(ZiskExecutionMetrics {
            duration: result.get_duration(),
            steps: result.get_execution_steps(),
            cost: result.get_execution_total_cost(),
            tx_count: None,
            gas_used: None,
        })
    }
}

#[expect(dead_code)]
fn parse_metrics(output: &str) -> Result<ZiskExecutionMetrics> {
    let mut steps = 0u64;
    let mut cost = 0u64;
    let mut tx_count = None;
    let mut gas_used = None;

    for line in output.lines() {
        if line.contains("STEPS")
            && let Some(val) = line.split_whitespace().last()
        {
            steps = val.replace(",", "").parse().unwrap_or(0);
        }
        if line.contains("TOTAL")
            && line.contains("100.00%")
            && let Some(val) = line.split_whitespace().nth(1)
        {
            cost = val.replace(",", "").parse().unwrap_or(0);
        }
        if line.contains("- Transaction Count:")
            && let Some(val) = line.split(':').next_back()
        {
            tx_count = val.trim().replace(",", "").parse().ok();
        }
        if line.contains("- Gas Consumed:")
            && let Some(val) = line.split(':').next_back()
        {
            gas_used = val.trim().replace(",", "").parse().ok();
        }
    }

    Ok(ZiskExecutionMetrics {
        duration: Duration::default(), // Placeholder, should be set based on actual execution time
        steps,
        cost,
        tx_count,
        gas_used,
    })
}

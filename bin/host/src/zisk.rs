use anyhow::{Context, Ok, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use zisk_sdk::{Asm, ElfBinary, Emu, ProverClient, ZiskProgramPK, ZiskProver, ZiskStdin};

#[derive(Debug, serde::Serialize)]
pub struct ZiskExecutionMetrics {
    pub steps: u64,
    pub cost: u64,
    pub tx_count: Option<u64>,
    pub gas_used: Option<u64>,
}

/// ZisK client backend wrapper
pub enum ZiskClient {
    Emu(ZiskProver<Emu>),
    Asm(ZiskProver<Asm>),
}

pub struct Zisk {
    pub elf: ElfBinary,
    pub zisk_client: Option<ZiskClient>,
    pub ziskemu: Option<PathBuf>,
    pub pk: Option<ZiskProgramPK>,
}

impl Zisk {
    pub fn new(elf: ElfBinary) -> Self {
        Self {
            elf,
            ziskemu: None,
            zisk_client: None,
            pk: None,
        }
    }

    pub fn with_ziskemu(mut self, ziskemu: impl Into<PathBuf>) -> Self {
        self.ziskemu = Some(ziskemu.into());
        self
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
            ZiskClient::Emu(prover)
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
            ZiskClient::Asm(prover)
        };

        self.zisk_client = Some(client);

        Ok(self)
    }

    /// Execute the guest program and return metrics
    pub fn ziskemu(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let ziskemu = self
            .ziskemu
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ZisK Emulator path is required for execution"))?;
        let elf_path = self
            .elf
            .path()
            .ok_or_else(|| anyhow::anyhow!("ELF path not available"))?;
        let output = Command::new(ziskemu)
            .arg("-e")
            .arg(&elf_path)
            .arg("-i")
            .arg(input_file)
            .arg("--stats")
            .output()
            .context("Failed to run ziskemu")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ziskemu execute failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_metrics(&stdout)
    }

    /// Execute and verify constraints
    pub fn verify_constraints(&self, input_file: &Path) -> Result<()> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let pk = self
            .pk
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proving key is not set up"))?;

        let client = self
            .zisk_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        match client {
            ZiskClient::Emu(prover) => prover.verify_constraints(pk, stdin)?,
            ZiskClient::Asm(prover) => prover.verify_constraints(pk, stdin)?,
        };

        Ok(())
    }

    pub fn execute(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let pk = self
            .pk
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proving key is not set up"))?;

        let client = self
            .zisk_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        let result = match client {
            ZiskClient::Emu(prover) => prover.execute(pk, stdin)?,
            ZiskClient::Asm(prover) => prover.execute(pk, stdin)?,
        };

        Ok(ZiskExecutionMetrics {
            steps: result.get_execution_steps(),
            cost: result.get_execution_total_cost(),
            tx_count: None,
            gas_used: None,
        })
    }
}

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
        steps,
        cost,
        tx_count,
        gas_used,
    })
}

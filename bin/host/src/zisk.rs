use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use zisk_sdk::{
    AsmOptions, EmbeddedClient, ExecutorKind, GuestProgram, ProverClient,
    VerifyConstraintsExtension, WitnessBuilderExt, ZiskStdin,
};

/// ZisK client
pub struct ZiskClient {
    pub program: GuestProgram,
    pub backend: Option<EmbeddedClient>,
    pub executor: ExecutorKind,
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
    pub fn new(program: GuestProgram) -> Self {
        Self {
            program,
            backend: None,
            executor: ExecutorKind::Emulator,
        }
    }

    pub fn with_proving_key(
        mut self,
        proving_key: Option<PathBuf>,
        use_emulator: bool,
        unlock_mapped_memory: bool,
        gpu: bool,
        no_aggregation: bool,
    ) -> Result<Self> {
        let mut builder = ProverClient::embedded();

        if !use_emulator {
            let asm_opts = if unlock_mapped_memory {
                AsmOptions::default().unlock_mapped_memory()
            } else {
                AsmOptions::default()
            };
            builder = builder.assembly().asm_options(asm_opts);
            self.executor = ExecutorKind::Assembly;
        }

        if let Some(pk) = proving_key {
            builder = builder.proving_key(pk);
        }

        if gpu {
            builder = builder.gpu();
        }

        if no_aggregation {
            builder = builder.no_aggregation();
        }

        self.backend = Some(builder.build().context("Failed to build EmbeddedClient")?);

        Ok(self)
    }

    /// Run ROM setup (call once before executing any inputs)
    pub async fn setup(&self) -> Result<()> {
        let client = self
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        client.setup(&self.program).run()?.await?;
        Ok(())
    }

    /// Execute the program with the given input and return execution metrics
    pub async fn execute(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let client = self
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        let result = client
            .execute(&self.program, stdin)
            .executor(self.executor)
            .run()?
            .await?;

        Ok(ZiskExecutionMetrics {
            duration: Duration::from_millis(result.get_execution_time()),
            steps: result.get_execution_steps(),
            cost: result.get_execution_cost(),
            tx_count: None,
            gas_used: None,
        })
    }

    /// Verify constraints for the program with the given input
    pub async fn verify_constraints(&self, input_file: &Path) -> Result<ZiskExecutionMetrics> {
        let stdin = ZiskStdin::from_file(input_file).context("Failed to load input file")?;

        let client = self
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client is not set up"))?;

        let result = client
            .verify_constraints(&self.program, stdin)
            .run()?
            .await?;

        Ok(ZiskExecutionMetrics {
            duration: Duration::from_millis(result.get_duration()),
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
        duration: Duration::default(),
        steps,
        cost,
        tx_count,
        gas_used,
    })
}

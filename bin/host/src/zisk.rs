use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::info;

use zisk_sdk::{
    AsmOptions, EmbeddedClient, EmbeddedClientBuilder, EmbeddedExecuteOnlyClient, ExecuteOutput,
    ExecutorKind, GuestProgram, ProverClient, VerifyConstraintsExtension, WitnessBuilderExt,
    ZiskHints, ZiskStdin,
};

enum Backend {
    Full(EmbeddedClient),
    ExecuteOnly(EmbeddedExecuteOnlyClient),
}

/// ZisK client
pub struct ZiskClient {
    pub program: GuestProgram,
    backend: Backend,
    executor: ExecutorKind,
    use_hints: bool,
}

/// Output metrics from ZisK execution
#[derive(Debug, serde::Serialize)]
pub struct ZiskExecutionMetrics {
    #[serde(skip)]
    pub duration: Duration,
    pub steps: u64,
    pub cost: Option<u64>,
    pub tx_count: Option<u64>,
    pub gas_used: Option<u64>,
}

impl std::fmt::Display for ZiskExecutionMetrics {
    /// Human-readable, one line: only the fields that are present, with grouped
    /// digits. `duration` is omitted (logged separately as the elapsed time).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = vec![format!("steps: {}", group_thousands(self.steps))];
        if let Some(cost) = self.cost {
            parts.push(format!("cost: {}", group_thousands(cost)));
        }
        if let Some(tx_count) = self.tx_count {
            parts.push(format!("txs: {}", group_thousands(tx_count)));
        }
        if let Some(gas_used) = self.gas_used {
            parts.push(format!("gas: {}", group_thousands(gas_used)));
        }
        write!(f, "{}", parts.join(", "))
    }
}

impl ZiskClient {
    pub fn for_execution(
        program: GuestProgram,
        use_emulator: bool,
        unlock_mapped_memory: bool,
        use_hints: bool,
    ) -> Result<Self> {
        let (builder, executor) =
            Self::embedded_builder(use_emulator, unlock_mapped_memory, use_hints)?;
        let client = builder
            .execute_only()
            .build()
            .context("Failed to build execute-only client")?;
        Ok(Self {
            program,
            backend: Backend::ExecuteOnly(client),
            executor,
            use_hints,
        })
    }

    pub fn for_proving(
        program: GuestProgram,
        proving_key: Option<PathBuf>,
        use_emulator: bool,
        unlock_mapped_memory: bool,
        gpu: bool,
        aggregate: bool,
        use_hints: bool,
    ) -> Result<Self> {
        let (mut builder, executor) =
            Self::embedded_builder(use_emulator, unlock_mapped_memory, use_hints)?;
        if let Some(pk) = proving_key {
            builder = builder.proving_key(pk);
        }
        if gpu {
            builder = builder.gpu();
        }
        if !aggregate {
            builder = builder.no_aggregation();
        }
        let client = builder.build().context("Failed to build EmbeddedClient")?;
        Ok(Self {
            program,
            backend: Backend::Full(client),
            executor,
            use_hints,
        })
    }

    fn embedded_builder(
        use_emulator: bool,
        unlock_mapped_memory: bool,
        use_hints: bool,
    ) -> Result<(EmbeddedClientBuilder, ExecutorKind)> {
        if use_hints && use_emulator {
            anyhow::bail!(
                "Running with hints requires the assembly backend; --emulator is not supported"
            );
        }

        let mut builder = ProverClient::embedded();
        let executor = if use_emulator {
            ExecutorKind::Emulator
        } else {
            let asm_opts = if unlock_mapped_memory {
                AsmOptions::default().unlock_mapped_memory()
            } else {
                AsmOptions::default()
            };
            builder = builder.assembly().asm_options(asm_opts);
            ExecutorKind::Assembly
        };

        Ok((builder, executor))
    }

    /// Run ROM setup (call once before executing any inputs)
    pub async fn setup(&self) -> Result<()> {
        match &self.backend {
            Backend::ExecuteOnly(client) => client.setup(&self.program, self.use_hints)?,
            Backend::Full(client) => {
                let mut setup = client.setup(&self.program);
                if self.use_hints {
                    setup = setup.with_hints();
                }
                setup.run()?.await?;
            }
        }
        Ok(())
    }

    /// Execute the program and return execution metrics.
    ///
    /// Exactly one source drives the run: `input_file` (normal execution) or
    /// `hints_file` (run with pre-generated hints — the input is carried by the
    /// hints, so stdin is empty). Hints require the assembly executor and a setup
    /// done with hints.
    pub async fn execute(
        &self,
        input_file: Option<&Path>,
        hints_file: Option<&Path>,
    ) -> Result<ZiskExecutionMetrics> {
        let stdin = match input_file {
            Some(file) => ZiskStdin::from_file(file).context("Failed to load input file")?,
            None => ZiskStdin::new(),
        };

        let hints = match hints_file {
            Some(path) => Some(ZiskHints::from_file(path).context("Failed to load hints file")?),
            None => None,
        };

        match &self.backend {
            Backend::ExecuteOnly(client) => {
                let result = client.execute(&self.program, stdin, hints)?;
                log_plan(&result);
                Ok(execute_metrics(
                    result.get_execution_time(),
                    result.get_execution_steps(),
                    result.get_execution_cost(),
                ))
            }
            Backend::Full(client) => {
                let mut request = client.execute(&self.program, stdin).executor(self.executor);
                if let Some(hints) = hints {
                    request = request.hints(hints);
                }
                let result = request.run()?.await?;
                log_plan(&result);
                Ok(execute_metrics(
                    result.get_execution_time(),
                    result.get_execution_steps(),
                    result.get_execution_cost(),
                ))
            }
        }
    }

    /// Verify constraints for the program.
    ///
    pub async fn verify_constraints(
        &self,
        input_file: Option<&Path>,
        hints_file: Option<&Path>,
    ) -> Result<ZiskExecutionMetrics> {
        let stdin = match input_file {
            Some(file) => ZiskStdin::from_file(file).context("Failed to load input file")?,
            None => ZiskStdin::new(),
        };

        let Backend::Full(client) = &self.backend else {
            anyhow::bail!("verify-constraints requires the full client, not execute-only");
        };

        let mut request = client.verify_constraints(&self.program, stdin);
        if let Some(path) = hints_file {
            let hints = ZiskHints::from_file(path).context("Failed to load hints file")?;
            request = request.hints(hints);
        }

        let result = request.run()?.await?;

        Ok(ZiskExecutionMetrics {
            duration: Duration::from_millis(result.get_duration()),
            steps: result.get_execution_steps(),
            cost: Some(result.get_execution_total_cost()),
            tx_count: None,
            gas_used: None,
        })
    }
}

/// Format an integer with thousands separators: `339736627` -> `339,736,627`.
fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn execute_metrics(time_ms: u64, steps: u64, cost: Option<u64>) -> ZiskExecutionMetrics {
    ZiskExecutionMetrics {
        duration: Duration::from_millis(time_ms),
        steps,
        cost,
        tx_count: None,
        gas_used: None,
    }
}

fn log_plan(output: &ExecuteOutput) {
    let Some(plan) = output.get_plan() else {
        return;
    };

    let total: usize = plan.iter().map(|e| e.count).sum();
    let mut by_group: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for entry in plan {
        by_group
            .entry(entry.airgroup_id)
            .or_default()
            .push(format!("{}: {}", entry.name, entry.count));
    }

    info!("--- PLAN SUMMARY --------------");
    for (airgroup_id, parts) in &by_group {
        let group_name = if *airgroup_id == 0 { "Zisk" } else { "Unknown" };
        info!(
            "{} | {} | Total instances: {}",
            group_name,
            parts.join(" | "),
            total
        );
    }
    info!("-----------------");
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
        cost: Some(cost),
        tx_count,
        gas_used,
    })
}

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, serde::Serialize)]
pub struct ExecutionMetrics {
    pub steps: u64,
    pub cost: u64,
    pub tx_count: Option<u64>,
    pub gas_used: Option<u64>,
}

pub fn execute(ziskemu: &Path, elf: &Path, input_file: &Path) -> Result<ExecutionMetrics> {
    let output = Command::new(ziskemu)
        .arg("-e")
        .arg(elf)
        .arg("-i")
        .arg(input_file)
        .arg("--stats")
        .output()
        .context("Failed to run ziskemu")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ziskemu failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_metrics(&stdout)
}

fn parse_metrics(output: &str) -> Result<ExecutionMetrics> {
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
        if line.contains("-Transaction Count:") {
            if let Some(val) = line.split(':').last() {
                tx_count = val.trim().replace(",", "").parse().ok();
            }
        }
        if line.contains("-Gas Consumed:") {
            if let Some(val) = line.split(':').last() {
                gas_used = val.trim().replace(",", "").parse().ok();
            }
        }
    }

    Ok(ExecutionMetrics {
        steps,
        cost,
        tx_count,
        gas_used,
    })
}

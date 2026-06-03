use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{error, info};

use input::{Client, ExecutionClient, create_client, generate_hints_to_file};
use zisk_sdk::{GuestProgram, ZiskStdin};

use crate::{
    cli::Action,
    zisk::{ZiskClient, ZiskExecutionMetrics},
};

pub struct BenchmarkRunner {
    action: Action,
    output_folder: Option<PathBuf>,
    force_rerun: bool,
    hints: Option<PathBuf>,
    gen_hints: bool,
    hints_out: Option<PathBuf>,
    native_client: Option<Box<dyn ExecutionClient>>,
    zisk_client: ZiskClient,
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkResult {
    test_name: String,
    time: f64,
    metrics: ZiskExecutionMetrics,
}

impl BenchmarkRunner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        elf: GuestProgram,
        action: Action,
        output_folder: Option<PathBuf>,
        force_rerun: bool,
        proving_key: Option<PathBuf>,
        emulator: bool,
        unlock_mapped_memory: bool,
        gpu: bool,
        hints: Option<PathBuf>,
        gen_hints: bool,
        hints_out: Option<PathBuf>,
        client: Client,
    ) -> Result<Self> {
        let use_hints = hints.is_some() || gen_hints;
        let zisk_client = match &action {
            Action::Execute => {
                ZiskClient::for_execution(elf, emulator, unlock_mapped_memory, use_hints)?
            }
            Action::VerifyConstraints => ZiskClient::for_proving(
                elf,
                proving_key,
                emulator,
                unlock_mapped_memory,
                gpu,
                false,
                use_hints,
            )?,
            Action::Prove => ZiskClient::for_proving(
                elf,
                proving_key,
                emulator,
                unlock_mapped_memory,
                gpu,
                true,
                use_hints,
            )?,
        };

        // The native client (reth/ethrex) generates hints in --gen-hints mode.
        let native_client = if gen_hints {
            Some(create_client(client))
        } else {
            None
        };

        // Default the hints output dir to <client>-hints, mirroring hints-gen.
        let hints_out = native_client
            .as_ref()
            .map(|nc| hints_out.unwrap_or_else(|| PathBuf::from(format!("{}-hints", nc.name()))));

        Ok(Self {
            action,
            output_folder,
            force_rerun,
            hints,
            gen_hints,
            hints_out,
            native_client,
            zisk_client,
        })
    }

    pub async fn run(
        &self,
        input_folder: Option<&Path>,
        include: Option<&[String]>,
        exclude: Option<&[String]>,
    ) -> Result<()> {
        #[cfg(not(zisk_hints))]
        if self.gen_hints {
            anyhow::bail!("--gen-hints requires building with RUSTFLAGS=\"--cfg zisk_hints\"");
        }

        if (self.hints.is_some() || self.gen_hints) && matches!(self.action, Action::Prove) {
            anyhow::bail!(
                "hints are only supported with the execute and verify-constraints actions"
            );
        }

        let mut files = if let Some(hints_root) = &self.hints {
            info!("Running with hints from {}", hints_root.display());
            collect_files(hints_root)?
        } else {
            let input_folder =
                input_folder.ok_or_else(|| anyhow::anyhow!("No input folder provided"))?;
            if self.gen_hints {
                let out = self
                    .hints_out
                    .as_ref()
                    .expect("hints_out is set whenever gen_hints is set");
                std::fs::create_dir_all(out).with_context(|| {
                    format!("Failed to create hints output dir: {}", out.display())
                })?;
                info!(
                    "Generating hints from {} into {}",
                    input_folder.display(),
                    out.display()
                );
            }
            collect_files(input_folder)?
        };

        if let Some(patterns) = include {
            info!("Include patterns: {:?}", patterns);
            files.retain(|file| {
                let name = file.to_string_lossy();
                patterns.iter().any(|p| name.contains(p))
            });
        }

        if let Some(patterns) = exclude {
            info!("Exclude patterns: {:?}", patterns);
            files.retain(|file| {
                let name = file.to_string_lossy();
                !patterns.iter().any(|p| name.contains(p))
            });
        }

        let total = files.len();
        info!("Found {} files to run", total);

        self.zisk_client.setup().await?;

        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        for (index, file) in files.iter().enumerate() {
            match self.run_single(file, index + 1, total).await {
                Ok(true) => passed += 1,
                Ok(false) => skipped += 1,
                Err(e) => {
                    error!("Failed to run benchmark for {}: {}", file.display(), e);
                    failed += 1;
                }
            }
        }

        info!("");
        info!(
            "Summary: {} passed, {} failed, {} skipped",
            passed, failed, skipped
        );

        // Propagate failures as a non-zero exit.
        if failed > 0 {
            anyhow::bail!("{failed} of {total} input(s) failed");
        }

        Ok(())
    }

    /// Resolve the `(input, hints)` sources for a work item, generating hints
    /// first in `--gen-hints` mode:
    /// - hints mode: the work file IS the hints (input carried by the hints).
    /// - gen-hints mode: the work file is an input; generate its hints natively,
    ///   then run with those (empty stdin).
    /// - otherwise: the work file is an input, run normally.
    fn prepare_sources(&self, work_file: &Path) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
        if self.hints.is_some() {
            Ok((None, Some(work_file.to_path_buf())))
        } else if self.gen_hints {
            let client = self
                .native_client
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("native client not initialized"))?;
            let out = self
                .hints_out
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("hints output dir not set"))?;
            let stem = work_file
                .file_stem()
                .with_context(|| format!("Input has no file stem: {}", work_file.display()))?;
            let hints_path = out.join(stem).with_extension("hints");

            let stdin = ZiskStdin::from_file(work_file).context("Failed to load input file")?;
            generate_hints_to_file(&stdin, hints_path.clone(), client)?;

            Ok((None, Some(hints_path)))
        } else {
            Ok((Some(work_file.to_path_buf()), None))
        }
    }

    async fn run_single(&self, work_file: &Path, current: usize, total: usize) -> Result<bool> {
        let test_name = work_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        match &self.action {
            Action::Execute => {
                if let Some(ref output_folder) = self.output_folder {
                    let filename = work_file.file_name().unwrap_or_default();
                    let output_file = output_folder.join(filename).with_extension("json");

                    if output_file.exists() && !self.force_rerun {
                        info!("[{}/{}] Skipping {}", current, total, test_name);
                        return Ok(false);
                    }
                }

                info!("[{}/{}] Running: {}", current, total, test_name);

                let (input_file, hints_file) = self.prepare_sources(work_file)?;
                let metrics = self
                    .zisk_client
                    .execute(input_file.as_deref(), hints_file.as_deref())
                    .await?;
                let elapsed = metrics.duration.as_secs_f64();

                info!("Execution metrics — {}", metrics);
                info!("[{}/{}] Completed in {:.2}s", current, total, elapsed);

                if let Some(ref output_folder) = self.output_folder {
                    let filename = work_file.file_name().unwrap_or_default();
                    let output_file = output_folder.join(filename).with_extension("json");

                    if let Some(parent) = output_file.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    let result = BenchmarkResult {
                        test_name: test_name.to_string(),
                        time: elapsed,
                        metrics,
                    };

                    let output_json = serde_json::to_string_pretty(&result)?;
                    fs::write(&output_file, output_json)?;
                }
            }

            Action::VerifyConstraints => {
                info!(
                    "[{}/{}] Verifying constraints: {}",
                    current, total, test_name
                );

                let (input_file, hints_file) = self.prepare_sources(work_file)?;
                let metrics = self
                    .zisk_client
                    .verify_constraints(input_file.as_deref(), hints_file.as_deref())
                    .await?;
                let elapsed = metrics.duration.as_secs_f64();

                info!("[{}/{}] PASSED in {:.2}s", current, total, elapsed);
            }

            Action::Prove => {
                unimplemented!("Prove action is not implemented yet");
            }
        }

        Ok(true)
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if root.is_dir() {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        files.push(sub_path);
                    }
                }
            } else if path.is_file() {
                files.push(path);
            }
        }
    } else {
        files.push(root.to_path_buf());
    }

    files.sort();
    Ok(files)
}

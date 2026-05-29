//! Native hint generation: run an [`ExecutionClient`] over a single input and
//! capture the hints into a sink (a file for batch generation, or a Unix socket
//! for streaming them to a live prover). Lives next to [`ExecutionClient`] so any
//! consumer of the trait can generate hints without depending on the host crate.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use zisk_sdk::ZiskStdin;

use crate::ExecutionClient;

/// Shared core: feed `stdin` as the native input, run `client.run()` under
/// `catch_unwind` with the sink opened by `init`, then tear it down with `deinit`.
/// `deinit` always runs — even if the client panics — so the sink flushes and
/// closes. Returns `(execution, total)` durations.
#[cfg(zisk_hints)]
fn run_with_hints(
    stdin: &ZiskStdin,
    client: &dyn ExecutionClient,
    init: impl FnOnce() -> Result<()>,
    deinit: impl FnOnce() -> Result<()>,
) -> Result<(Duration, Duration)> {
    ziskos::set_native_input(stdin.read_data());
    init()?;

    let t0 = std::time::Instant::now();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.run()));
    let execution = t0.elapsed();

    // Always tear down, then surface a run panic over a teardown error.
    let deinit_result = deinit();
    if let Err(e) = run_result {
        let msg = e
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| e.downcast_ref::<&str>().copied())
            .unwrap_or("unknown panic");
        return Err(anyhow::anyhow!("Block execution failed: {}", msg));
    }
    deinit_result?;

    Ok((execution, t0.elapsed()))
}

/// Generate hints for one input and write them to `output_path` (batch / file sink).
#[cfg(zisk_hints)]
pub fn generate_hints_to_file(
    stdin: &ZiskStdin,
    output_path: PathBuf,
    client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    let (execution, total) = run_with_hints(
        stdin,
        client,
        || {
            // SAFETY: single-threaded setup, before the run begins.
            unsafe { std::env::set_var("ZISK_HINTS_OUTPUT", &output_path) };
            ziskos::zkvm_init();
            Ok(())
        },
        || {
            ziskos::zkvm_deinit();
            Ok(())
        },
    )?;
    tracing::info!(
        "Written hints to {} (execution: {:.2?}, total: {:.2?})",
        output_path.display(),
        execution,
        total,
    );
    Ok((execution, total))
}

/// Generate hints for one input and stream them to a Unix socket (live / streaming
/// sink) so a prover can consume them while they are produced; no hints file is
/// written. `debug_file`, when `Some`, tees a copy to disk. `ready`, when `Some`,
/// is signalled once the socket is listening — before this call blocks waiting for
/// the prover to connect. Returns `(execution, total)` durations.
#[cfg(zisk_hints)]
pub fn generate_hints_to_socket(
    stdin: &ZiskStdin,
    socket_path: PathBuf,
    debug_file: Option<PathBuf>,
    write_flush_threshold: Option<usize>,
    ready: Option<tokio::sync::oneshot::Sender<()>>,
    client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    run_with_hints(
        stdin,
        client,
        move || ziskos::zkvm_init_socket(socket_path, debug_file, write_flush_threshold, ready),
        || ziskos::hints::close_hints(),
    )
}

#[cfg(not(zisk_hints))]
const NO_HINTS_MSG: &str =
    "Compiled without hints support. Rebuild with:\n  RUSTFLAGS=\"--cfg zisk_hints\" cargo build";

#[cfg(not(zisk_hints))]
pub fn generate_hints_to_file(
    _stdin: &ZiskStdin,
    _output_path: PathBuf,
    _client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    anyhow::bail!(NO_HINTS_MSG)
}

#[cfg(not(zisk_hints))]
pub fn generate_hints_to_socket(
    _stdin: &ZiskStdin,
    _socket_path: PathBuf,
    _debug_file: Option<PathBuf>,
    _write_flush_threshold: Option<usize>,
    _ready: Option<tokio::sync::oneshot::Sender<()>>,
    _client: &dyn ExecutionClient,
) -> Result<(Duration, Duration)> {
    anyhow::bail!(NO_HINTS_MSG)
}

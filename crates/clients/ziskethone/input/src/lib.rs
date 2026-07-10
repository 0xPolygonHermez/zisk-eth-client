use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use guest_common::chain::chain_name;
use zisk_sdk::ZiskStdin;

pub use guest_ziskethone as guest;

pub use input_core::RpcConfig;
use input_core::{BlockStats, ExecutionClient};

/// Default ancestor depth for the PreviousBlocks section. The EVM BLOCKHASH
/// opcode reaches back 256, matching `rust-input-gen`'s CLI default.
const DEFAULT_ANCESTORS: u64 = 256;

#[derive(Default)]
pub struct ZiskEthOneClient;

#[async_trait]
impl ExecutionClient for ZiskEthOneClient {
    fn name(&self) -> &'static str {
        "ziskethone"
    }

    fn display_name(&self) -> &'static str {
        "ZiskEthOne"
    }

    async fn from_rpc(
        &self,
        config: &RpcConfig,
        block_number: u64,
    ) -> Result<(ZiskStdin, BlockStats)> {
        if !config.headers.is_empty() {
            static WARNED: AtomicBool = AtomicBool::new(false);
            if !WARNED.swap(true, Ordering::Relaxed) {
                tracing::warn!(
                    "--rpc-headers is not honored by the ziskethone client; ignoring {} header(s)",
                    config.headers.len()
                );
            }
        }

        let (bytes, s) = rust_input_gen::live::fetch_and_build_with_stats(
            &config.url,
            block_number,
            DEFAULT_ANCESTORS,
        )
        .await
        .map_err(|e| anyhow!("ziskethone input generation failed: {e}"))?;

        let stdin = ZiskStdin::new();
        stdin.write_slice(&bytes);

        let stats = BlockStats {
            chain_name: chain_name(s.chain_id),
            block_number: s.block_number,
            tx_count: s.tx_count,
            gas_used: s.gas_used,
        };

        Ok((stdin, stats))
    }

    /// Run ziskethone's C++ EVM natively in-process over the block container.
    ///
    /// Mirrors zilkworm's `run()`: the input is read from ziskos's native-input
    /// buffer (the hints harness fills it via `ziskos::set_native_input` before
    /// calling `run`), so the client stays stateless. `ziskos::io::read_slice`
    /// returns the `ZEG0` container bytes, which we hand to the C++ EVM via FFI.
    fn run(&self) {
        let input = ziskos::io::read_slice();
        let block_hash = guest_ziskethone::run(&input);
        let hex: String = block_hash.iter().map(|b| format!("{b:02x}")).collect();
        tracing::info!("ziskethone run complete; execution block hash: 0x{hex}");
    }

    /// `run()` runs the C++ EVM over FFI as a native input checker; it does not
    /// emit ZisK hints. The hints harness rejects this client rather than
    /// writing an empty hints file.
    fn emits_hints(&self) -> bool {
        false
    }
}

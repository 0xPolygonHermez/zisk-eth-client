use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tempfile::TempDir;
use zisk_sdk::ZiskStdin;

use guest_common::chain::chain_name;
use guest_zilkworm::{fetch_block_and_witness, FetchRequest};

use super::client::{BlockStats, ExecutionClient, RpcConfig};

#[derive(Default)]
pub struct ZilkwormClient;

#[async_trait]
impl ExecutionClient for ZilkwormClient {
    fn name(&self) -> &'static str {
        "zilkworm"
    }

    fn display_name(&self) -> &'static str {
        "Zilkworm"
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
                    "--rpc-headers is not honored by the zilkworm client; ignoring {} header(s)",
                    config.headers.len()
                );
            }
        }

        // zilkworm's fetcher writes to disk; contain it in a tempdir.
        let tempdir = TempDir::new().context("Failed to create tempdir for zilkworm fetch")?;
        let outcome = fetch_block_and_witness(FetchRequest {
            rpc_url: &config.url,
            block_number: Some(block_number),
            data_dir: tempdir.path().to_owned(),
            save_all_responses: false,
            build_eth_test: false,
            geth: false,
        })
        .await
        .map_err(|e| anyhow!("zilkworm fetch_block_and_witness failed: {e}"))?;

        let unified_rlp = std::fs::read(&outcome.unified_rlp_path).with_context(|| {
            format!(
                "Failed to read zilkworm unified RLP at {}",
                outcome.unified_rlp_path.display()
            )
        })?;

        // Guest payload: [is_test=0][unifiedBlockAndStateRlp bytes].
        let mut payload = Vec::with_capacity(1 + unified_rlp.len());
        payload.push(0u8);
        payload.extend_from_slice(&unified_rlp);

        let stdin = ZiskStdin::new();
        stdin.write_slice(&payload);

        let stats = BlockStats {
            chain_name: chain_name(outcome.chain_id),
            block_number: outcome.block_number,
            tx_count: outcome.tx_count as usize,
            gas_used: outcome.gas_used,
        };

        Ok((stdin, stats))
    }

    fn run(&self) {
        guest_zilkworm::run();
    }
}

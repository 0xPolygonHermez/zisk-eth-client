use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use tokio_util::sync::CancellationToken;
use tracing::info;

use alloy_provider::{Provider, ProviderBuilder};

use super::{InputProvider, ProviderKind};
use crate::{client::InputGenClient, processor::ProcessingTracker};

#[derive(Debug, Clone, Args)]
pub struct RpcProvider {
    /// RPC URL to use
    #[arg(short = 'u', long)]
    rpc_url: String,

    /// Number of last blocks to fetch (default: 1 if no other block selection method is used)
    #[arg(short = 'l', long, group = "block_selection")]
    last_n_blocks: Option<usize>,

    /// Specific block number to fetch
    #[arg(short = 'b', long, group = "block_selection")]
    block: Option<u64>,

    /// Fetch blocks in a range (inclusive)
    #[arg(short = 'r', long, num_args = 2, value_names = ["START", "END"], group = "block_selection")]
    range_of_blocks: Option<Vec<u64>>,

    /// Listen for new blocks
    #[arg(short = 'f', long, default_value_t = false, group = "block_selection")]
    follow: bool,
}

#[async_trait::async_trait]
impl InputProvider for RpcProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Rpc
    }

    async fn generate_inputs(&self, client: &dyn InputGenClient, output: &Path) -> Result<()> {
        if !client.supports_provider(self.kind()) {
            anyhow::bail!("{} doesn't support RPC provider", client.display_name());
        }

        info!(
            "Generating inputs for the {} client...",
            client.display_name()
        );

        if self.follow {
            self.follow_new_blocks(output, client).await
        } else {
            self.process_batch(output, client).await
        }
    }
}

impl RpcProvider {
    async fn process_batch(&self, output: &Path, client: &dyn InputGenClient) -> Result<()> {
        let block_numbers: Vec<u64> = if let Some(block_num) = self.block {
            vec![block_num]
        } else if let Some(range) = &self.range_of_blocks {
            if range.len() != 2 {
                anyhow::bail!("Range requires exactly 2 values: START and END");
            }
            let (start, end) = (range[0], range[1]);
            if start > end {
                anyhow::bail!("Range START ({}) must be <= END ({})", start, end);
            }
            (start..=end).collect()
        } else {
            let n = self.last_n_blocks.unwrap_or(1);
            if n == 0 {
                info!("No blocks to process (last_n_blocks = 0)");
                return Ok(());
            }
            let provider = self.connect_provider().await?;
            let latest = fetch_latest_block_number(&provider).await?;
            let start = latest.saturating_sub(n as u64 - 1);
            (start..=latest).collect()
        };

        info!(
            "Processing {} block(s): {:?}",
            block_numbers.len(),
            block_numbers
        );

        let mut tracker = ProcessingTracker::new(client.display_name());
        for block_num in block_numbers {
            let name = format!("Block #{}", block_num);
            match self.process_block(block_num, output, client).await {
                Ok(_) => tracker.record_success(&name),
                Err(e) => tracker.record_error(&name, &e),
            }
        }
        tracker.log_summary();
        if tracker.error_count() > 0 {
            anyhow::bail!("{} block(s) failed", tracker.error_count());
        }
        Ok(())
    }

    async fn follow_new_blocks(&self, output: &Path, client: &dyn InputGenClient) -> Result<()> {
        info!("Following new blocks (press Ctrl+C to stop)...");

        let stop = CancellationToken::new();
        let stop_clone = stop.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            info!("Received Ctrl+C, stopping...");
            stop_clone.cancel();
        });

        let provider = self.connect_provider().await?;
        let mut tracker = ProcessingTracker::new(client.display_name());
        let mut next_block_num = fetch_latest_block_number(&provider).await?;

        loop {
            if stop.is_cancelled() {
                break;
            }

            let latest = fetch_latest_block_number(&provider).await?;
            for block_num in next_block_num..=latest {
                if stop.is_cancelled() {
                    break;
                }
                let name = format!("Block #{}", block_num);
                match self.process_block(block_num, output, client).await {
                    Ok(_) => tracker.record_success(&name),
                    Err(e) => tracker.record_error(&name, &e),
                }
            }

            next_block_num = latest + 1;

            tokio::select! {
                _ = stop.cancelled() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(6)) => {}
            }
        }

        tracker.log_summary();
        Ok(())
    }

    /// Delegate to the client's own RPC path, then save the resulting stdin
    /// using a filename derived from the returned [`BlockStats`].
    async fn process_block(
        &self,
        block_num: u64,
        output: &Path,
        client: &dyn InputGenClient,
    ) -> Result<()> {
        let (stdin, stats) = client.from_rpc(&self.rpc_url, block_num).await?;
        stdin.save(&output.join(stats.output_filename(client.name())))?;
        Ok(())
    }

    async fn connect_provider(&self) -> Result<impl Provider> {
        ProviderBuilder::new()
            .connect(&self.rpc_url)
            .await
            .context("Failed to connect to RPC provider")
    }
}

async fn fetch_latest_block_number<P: Provider>(provider: &P) -> Result<u64> {
    provider
        .get_block_number()
        .await
        .context("Failed to fetch latest block number")
}

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use tracing::info;
use witness_generator::{
    rpc_generator::{RpcBlocksAndWitnessesBuilder, RpcFlatHeaderKeyValues},
    FixtureGenerator,
};

use crate::common::{generate_reth_inputs, read_fixtures_from_path};
use crate::OutputFormat;

/// Process blocks from an RPC endpoint to generate reth inputs.
pub async fn process_rpc(
    rpc_url: String,
    rpc_header: Option<Vec<String>>,
    block: Option<u64>,
    last_n_blocks: Option<usize>,
    follow: bool,
    output: &Path,
    format: OutputFormat,
) -> Result<()> {
    info!("Connecting to RPC: {}", rpc_url);

    let mut builder = RpcBlocksAndWitnessesBuilder::new(rpc_url);

    if let Some(rpc_header) = rpc_header {
        let headers = RpcFlatHeaderKeyValues::new(rpc_header)
            .try_into()
            .context("Failed to parse RPC headers")?;
        builder = builder.with_headers(headers);
    }

    if follow {
        let stop = CancellationToken::new();
        builder = builder.listen(stop.clone());

        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Stopping...");
                    stop.cancel();
                }
            }
        });
    } else if let Some(block_num) = block {
        builder = builder.block(block_num);
    } else {
        let n_blocks = last_n_blocks.unwrap_or(1);
        if n_blocks == 0 {
            return Err(anyhow!("Number of blocks must be greater than 0"));
        }
        builder = builder.last_n_blocks(n_blocks);
    }

    let generator = builder
        .build()
        .await
        .context("Failed to build RPC generator")?;

    // Generate fixtures to a temp directory, then convert to reth inputs
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    let count = generator
        .generate_to_path(temp_dir.path())
        .await
        .context("Failed to generate RPC fixtures")?;

    info!(
        "Generated {} RPC fixtures, converting to reth inputs...",
        count
    );

    let fixtures = read_fixtures_from_path(temp_dir.path())?;
    generate_reth_inputs(&fixtures, output, format)
}

//! Host-side input handling for the reth guest.
//!
//! The guest consumes the execution-specs stateless input
//! (`schema_id || SSZ(StatelessInput)`, tests-zkevm v21.0.1) as a single
//! `ZiskStdin` frame, and writes the `StatelessValidationResult`.
//! [`block_input`] builds such inputs from block data, [`rpc`] fetches that data
//! from an execution-layer node (re-executing natively when the EIP-7685
//! requests are needed), and EEST fixtures already carry the bytes verbatim.

pub mod block_input;
pub mod native;
pub mod rpc;

use anyhow::{Context, Result};
use async_trait::async_trait;

use zisk_sdk::ZiskStdin;

use guest_common::chain::chain_name;
use guest_reth::InputSummary;

pub use block_input::BlockInput;
pub use guest_reth as guest;
pub use input_core::RpcConfig;
use input_core::{first_frame, BlockStats, ExecutionClient, InputStats};

#[derive(Default)]
pub struct RethClient;

impl RethClient {
    /// Wrap stateless input bytes as the guest's single-frame stdin. The bytes
    /// are decoded first so a malformed input fails here, not in the guest.
    pub fn stdin_from_bytes(&self, input: &[u8]) -> Result<ZiskStdin> {
        InputSummary::decode(input)
            .map_err(|err| anyhow::anyhow!("invalid stateless input: {err:?}"))?;
        let stdin = ZiskStdin::new();
        stdin.write_slice(input);
        Ok(stdin)
    }

    /// Stdin plus block stats for an input built by
    /// [`block_input::build_block_input`].
    pub fn stdin_from_input(&self, input: &BlockInput) -> Result<(ZiskStdin, BlockStats)> {
        let stdin = self
            .stdin_from_bytes(&input.stateless_input_bytes)
            .with_context(|| format!("block {}", input.block_number))?;
        Ok((stdin, input.block_stats()))
    }
}

#[async_trait]
impl ExecutionClient for RethClient {
    fn name(&self) -> &'static str {
        "reth"
    }

    fn display_name(&self) -> &'static str {
        "Reth"
    }

    async fn from_rpc(
        &self,
        config: &RpcConfig,
        block_number: u64,
    ) -> Result<(ZiskStdin, BlockStats)> {
        let input = rpc::fetch_block_input(config, block_number).await?;
        self.stdin_from_input(&input)
    }

    fn run(&self) {
        guest_reth::run();
    }

    /// The 43-byte `StatelessValidationResult` for a valid block is a function
    /// of the input alone (payload root, chain id, schema id).
    fn expected_output(&self, stdin: &ZiskStdin) -> Result<Option<Vec<u8>>> {
        let buf = stdin.read_data();
        let output = guest_reth::expected_output(first_frame(&buf)?)
            .map_err(|err| anyhow::anyhow!("failed to decode the stateless input: {err:?}"))?;
        Ok(Some(output))
    }

    /// The input is a single frame holding the stateless input bytes.
    fn input_stats(&self, stdin: &ZiskStdin) -> Result<Option<InputStats>> {
        let buf = stdin.read_data();
        let summary = InputSummary::decode(first_frame(&buf)?)
            .map_err(|err| anyhow::anyhow!("failed to decode the stateless input: {err:?}"))?;
        Ok(Some(InputStats {
            block_number: summary.block_number,
            tx_count: summary.tx_count,
            gas_used: summary.gas_used,
        }))
    }
}

impl BlockInput {
    pub fn block_stats(&self) -> BlockStats {
        BlockStats {
            chain_name: chain_name(self.chain_id),
            block_number: self.block_number,
            tx_count: self.tx_count,
            gas_used: self.gas_used,
        }
    }
}

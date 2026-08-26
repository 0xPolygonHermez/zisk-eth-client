use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use async_trait::async_trait;
use url::Url;
use zisk_sdk::ZiskStdin;

use ethrex_common::types::block_execution_witness::{decode_witness_headers, RpcExecutionWitness};
use ethrex_common::types::Block;
use ethrex_common::NativeCrypto;
use ethrex_config::networks::Network;
use ethrex_rpc::types::{block::RpcBlock, block_identifier::BlockIdentifier};
use ethrex_rpc::EthClient;

use guest_common::chain::chain_name;
use guest_ethrex::EthrexInput;

pub use guest_ethrex as guest;
pub use input_core::RpcConfig;
use input_core::{first_frame, BlockStats, ExecutionClient, InputStats};

#[derive(Default)]
pub struct EthrexClient;

impl EthrexClient {
    fn build_stdin(&self, input: &EthrexInput) -> Result<ZiskStdin> {
        let bytes = input
            .serialize()
            .context("Failed to serialize EthrexInput")?;
        let stdin = ZiskStdin::new();
        stdin.write_slice(&bytes);
        Ok(stdin)
    }
}

#[async_trait]
impl ExecutionClient for EthrexClient {
    fn name(&self) -> &'static str {
        "ethrex"
    }

    fn display_name(&self) -> &'static str {
        "Ethrex"
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
                    "--rpc-headers is not honored by the ethrex client (EthClient has no header support); ignoring {} header(s)",
                    config.headers.len()
                );
            }
        }

        let url = Url::parse(&config.url).context("Invalid RPC URL")?;
        let rpc_client = EthClient::new(url).context("Failed to create EthClient")?;

        let chain_id = rpc_client.get_chain_id().await?.as_u64();
        let chain_config = Network::try_from(chain_id)
            .map_err(|e| anyhow::anyhow!("Unsupported chain ID {chain_id}: {e}"))?
            .get_genesis()
            .context("Failed to get genesis config")?
            .config;

        let rpc_block: RpcBlock = rpc_client
            .get_block_by_number(BlockIdentifier::Number(block_number), true)
            .await
            .with_context(|| format!("Failed to fetch block {block_number}"))?;

        let rpc_witness: RpcExecutionWitness = rpc_client
            .get_witness(BlockIdentifier::Number(block_number), None)
            .await
            .with_context(|| format!("Failed to fetch witness for block {block_number}"))?;

        let block: Block = rpc_block
            .try_into()
            .map_err(|e: String| anyhow::anyhow!("{e}"))?;

        let stats = BlockStats {
            chain_name: chain_name(chain_id),
            block_number,
            tx_count: block.body.transactions.len(),
            gas_used: block.header.gas_used,
        };

        // v20 pushes header decoding + crypto injection to the caller.
        // Host-side runs natively, so use `NativeCrypto`.
        let decoded_headers = decode_witness_headers(&rpc_witness.headers)
            .map_err(|e| anyhow::anyhow!("Failed to decode witness headers: {e}"))?;
        let witness = rpc_witness
            .into_execution_witness(chain_config, block_number, &decoded_headers, &NativeCrypto)
            .map_err(|e| anyhow::anyhow!("Failed to convert execution witness: {e}"))?;

        let input = EthrexInput::new(block, witness);
        let stdin = self.build_stdin(&input)?;
        Ok((stdin, stats))
    }

    fn run(&self) {
        guest_ethrex::run();
    }

    /// Ethrex writes block and witness as one rkyv frame, so this pays for a
    /// full deserialize of the witness too.
    fn input_stats(&self, stdin: &ZiskStdin) -> Result<Option<InputStats>> {
        let buf = stdin.read_data();
        let input = EthrexInput::deserialize(first_frame(&buf)?)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize EthrexInput: {e}"))?;
        let block = input.block();
        Ok(Some(InputStats {
            block_number: block.header.number,
            tx_count: block.body.transactions.len(),
            gas_used: block.header.gas_used,
        }))
    }
}

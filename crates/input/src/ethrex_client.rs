use anyhow::{Context, Result};
use async_trait::async_trait;
use url::Url;
use zisk_sdk::ZiskStdin;

use ethrex_common::types::block_execution_witness::RpcExecutionWitness;
use ethrex_config::networks::Network;
use ethrex_rpc::debug::execution_witness::execution_witness_from_rpc_chain_config;
use ethrex_rpc::types::{block::RpcBlock, block_identifier::BlockIdentifier};
use ethrex_rpc::EthClient;

use guest_ethrex::EthrexInput;

use super::client::ExecutionClient;

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

    async fn from_rpc(&self, rpc_url: &str, block_number: u64) -> Result<ZiskStdin> {
        let url = Url::parse(rpc_url).context("Invalid RPC URL")?;
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

        let block = rpc_block
            .try_into()
            .map_err(|e: String| anyhow::anyhow!("{e}"))?;
        let witness =
            execution_witness_from_rpc_chain_config(rpc_witness, chain_config, block_number)
                .context("Failed to convert execution witness")?;

        let input = EthrexInput::new(block, witness);
        self.build_stdin(&input)
    }

    fn run(&self) {
        guest_ethrex::run();
    }
}

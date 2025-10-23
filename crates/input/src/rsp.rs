use std::sync::Arc;

use async_trait::async_trait;
use rsp_host_executor::EthHostExecutor;
use rsp_primitives::genesis::Genesis;
use rsp_provider::create_provider;

use crate::types::{InputGenerator, InputGeneratorConfig, InputGeneratorResult, Network};

pub struct RspInputGenerator {
    pub config: InputGeneratorConfig,
}

impl RspInputGenerator {
    pub fn new(config: InputGeneratorConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl InputGenerator for RspInputGenerator {
    async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult> {
        // Create the RPC provider
        let provider = create_provider(self.config.rpc_url.clone());

        let genesis = match self.config.network {
            Some(Network::Mainnet) => {
                Genesis::Mainnet
            }
            Some(Network::Sepolia) => {
                Genesis::Sepolia
            }
            None => {
                Genesis::Mainnet
            }
        };

        let executor = EthHostExecutor::eth(
            Arc::new(
                (&genesis).try_into().expect("Failed to convert genesis block into the required type"),
            ),
            None,
        );

        let input = executor
            .execute(block_number, &provider, genesis.clone(), None, false)
            .await
            .expect("Failed to execute client");

        let input_bytes = bincode::serialize(&input)
            .expect("Failed to serialize input");

        Ok(InputGeneratorResult {
            input: input_bytes,
            gas_used: input.current_block.gas_used,
            tx_count: input.current_block.body.transactions.len().try_into().unwrap(),
        })
    }

    fn get_config(&self) -> &InputGeneratorConfig {
        &self.config
    }
}

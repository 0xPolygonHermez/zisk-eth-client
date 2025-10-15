use std::sync::Arc;

use anyhow::{anyhow};
use alloy_provider::{Provider, ProviderBuilder};
use async_trait::async_trait;
use zeth_host::BlockProcessor;

use crate::types::{InputGenerator, InputGeneratorConfig, InputGeneratorResult};

pub struct ZethInputGenerator {
    pub config: InputGeneratorConfig,
}

impl ZethInputGenerator {
    pub fn new(config: InputGeneratorConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl InputGenerator for ZethInputGenerator {
    async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult> {
        let provider = ProviderBuilder::new().connect(self.config.rpc_url.as_str()).await?;
        let processor = BlockProcessor::new(Arc::new(provider)).await?;

        // First, get the block header to determine the canonical hash for caching.
        let header = processor
            .provider()
            .get_block(block_number.into())
            .await?
            .ok_or_else(|| anyhow!("block {} not found", block_number))?
            .header;

        let (input, _) = processor.create_input(header.hash).await?;

        let input_bytes = bincode::serialize(&input)
            .expect("Failed to serialize input");

        let input_path =self.save_input_to_file(
            block_number,
            input.block.header.gas_used,
            input.block.body.transactions.len().try_into().unwrap(),
            input_bytes)?;

        Ok(InputGeneratorResult {
            input_file_path: input_path,
            gas_used: input.block.header.gas_used,
            tx_count: input.block.body.transactions.len().try_into().unwrap(),
        })
    }

    fn get_config(&self) -> &InputGeneratorConfig {
        &self.config
    }
}

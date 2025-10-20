use alloy_consensus::{private::serde};
use alloy::{rpc::types::debug::ExecutionWitness};
use anyhow::{anyhow, Context, Result};
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use async_trait::async_trait;
use k256::ecdsa::VerifyingKey;
use reth_ethereum_primitives::{Block, TransactionSigned};

use crate::types::{InputGenerator, InputGeneratorConfig, InputGeneratorResult};

/// `StatelessInput` is a convenience structure for serializing the input needed
/// for the stateless validation function.
#[serde_with::serde_as]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Input {
    /// The block being executed in the stateless validation function
    #[serde_as(
        as = "reth_primitives_traits::serde_bincode_compat::Block<reth_ethereum_primitives::TransactionSigned, alloy_consensus::Header>"
    )]
    pub block: Block,
    /// List of signing public keys for each transaction in the block.
    pub signers: Vec<VerifyingKey>,
    /// `ExecutionWitness` for the stateless validation function
    pub witness: ExecutionWitness,
}

pub struct ZethInputGenerator {
    pub config: InputGeneratorConfig,
}

impl ZethInputGenerator {
    pub fn new(config: InputGeneratorConfig) -> Self {
        Self { config }
    }
}

/// Recovers the signing [`VerifyingKey`] from each transaction's signature.
pub fn recover_signers<'a, I>(txs: I) -> Result<Vec<VerifyingKey>>
where
    I: IntoIterator<Item = &'a TransactionSigned>,
{
    txs.into_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect::<Result<Vec<_>, _>>()
}

#[async_trait]
impl InputGenerator for ZethInputGenerator {
    async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult> {
        let provider = ProviderBuilder::new().connect(self.config.rpc_url.as_str()).await?;

        // First, get the block header to determine the canonical hash for caching.
        let header = provider
            .get_block(block_number.into())
            .await?
            .ok_or_else(|| anyhow!("block {} not found", block_number))?
            .header;

        // let (input, _) = processor.create_input(header.hash).await?;

        let block_id = header.hash.into();
        let rpc_block = provider
            .get_block(block_id)
            .full()
            .await?
            .with_context(|| format!("block {block_id} not found"))?;
        let witness = provider.debug_execution_witness(rpc_block.number().into()).await?;
        let block = reth_ethereum_primitives::Block::from(rpc_block);
        let signers = recover_signers(block.body.transactions())?;

        let input = Input {
            block,
            signers,
            witness: ExecutionWitness {
                state: witness.state,
                codes: witness.codes,
                keys: vec![], // keys are not used
                headers: witness.headers,
            },
        };

        let input_bytes = bincode::serialize(&input)
            .expect("Failed to serialize input");

        Ok(InputGeneratorResult {
            input: input_bytes,
            gas_used: input.block.header.gas_used,
            tx_count: input.block.body.transactions.len().try_into().unwrap(),
        })
    }

    fn get_config(&self) -> &InputGeneratorConfig {
        &self.config
    }
}

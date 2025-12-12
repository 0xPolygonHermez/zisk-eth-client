use alloy::rpc::types::debug::ExecutionWitness;
use alloy_consensus::private::serde;
use alloy_provider::{ext::DebugApi, Provider, ProviderBuilder};
use anyhow::{Context, Result};
use async_trait::async_trait;
use k256::ecdsa::VerifyingKey;
use rayon::prelude::*;
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

// /// Recovers the signing [`VerifyingKey`] from each transaction's signature.
// pub fn recover_signers<'a, I>(txs: I) -> Result<Vec<VerifyingKey>>
// where
//     I: IntoIterator<Item = &'a TransactionSigned>,
// {
//     txs.into_iter()
//         .enumerate()
//         .map(|(i, tx)| {
//             tx.signature()
//                 .recover_from_prehash(&tx.signature_hash())
//                 .with_context(|| format!("failed to recover signature for tx #{i}"))
//         })
//         .collect::<Result<Vec<_>, _>>()
// }

// Recovers the signing [`VerifyingKey`] from each transaction's signature, in parallel.
pub fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<VerifyingKey>> {
    txs.par_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect()
}

#[async_trait]
impl InputGenerator for ZethInputGenerator {
    async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult> {
        let start_rpc_connect = std::time::Instant::now();
        let provider = ProviderBuilder::new()
            .connect(self.config.rpc_url.as_str())
            .await?;
        let time_rpc_connect = start_rpc_connect.elapsed();

        let start_block_fetch = std::time::Instant::now();
        let rpc_block = provider
            .get_block(block_number.into())
            .full()
            .await?
            .with_context(|| format!("block {block_number} not found"))?;
        let time_block_fetch = start_block_fetch.elapsed();

        let start_witness_fetch = std::time::Instant::now();
        let witness = provider
            .debug_execution_witness(rpc_block.number().into())
            .await?;
        let time_witness_fetch = start_witness_fetch.elapsed();

        let block = reth_ethereum_primitives::Block::from(rpc_block);

        let start_recover_signers = std::time::Instant::now();
        let signers = recover_signers(block.body.transactions.as_slice())?;
        let time_recover_signers = start_recover_signers.elapsed();

        let start_serialize_input = std::time::Instant::now();
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

        let input_bytes = bincode::serialize(&input).expect("Failed to serialize input");
        let time_serialize_input = start_serialize_input.elapsed();

        println!("input generation timings for block {block_number}: rpc connect: {:?}, block fetch: {:?}, witness fetch: {:?}, recover signers: {:?}, serialize input: {:?}",
            time_rpc_connect,
            time_block_fetch,
            time_witness_fetch,
            time_recover_signers,
            time_serialize_input,
        );

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

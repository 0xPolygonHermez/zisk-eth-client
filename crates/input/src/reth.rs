use alloy_eips::BlockNumberOrTag;
use alloy_rpc_types_eth::{Block, Header, Receipt, Transaction, TransactionRequest};
use anyhow::Context;
use jsonrpsee::http_client::HttpClientBuilder;
use reth_chainspec::{Chain, HOLESKY, HOODI, NamedChain, SEPOLIA, mainnet_chain_config};
use reth_ethereum_primitives::TransactionSigned;
use reth_rpc_api::{DebugApiClient, EthApiClient};
use reth_stateless::StatelessInput;
use stateless_validator_reth::guest::StatelessValidatorRethInput;
use witness_generator::StatelessValidationFixture;

use crate::types::{GuestProgram, InputGenerator, InputGeneratorResult};

impl InputGenerator {
    pub async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult> {
        println!(
            "Generating input for block {}, guest: zec-reth",
            block_number
        );

        // Build HTTP client
        let client = HttpClientBuilder::default()
            .max_response_size(1 << 30)
            .build(&self.rpc_url)
            .with_context(|| "Failed to build HTTP client")?;

        // Fetch chain ID and determine chain config
        let chain_id = EthApiClient::<(), (), (), (), (), ()>::chain_id(&client)
            .await
            .with_context(|| "Failed to fetch chain ID")?
            .with_context(|| "Chain ID not found")?;

        let chain = Chain::from_id(chain_id.to());
        let (chain_config, chain_name) = match chain.named() {
            Some(NamedChain::Mainnet) => (mainnet_chain_config(), "mainnet"),
            Some(NamedChain::Sepolia) => (SEPOLIA.genesis.config.clone(), "sepolia"),
            Some(NamedChain::Hoodi) => (HOODI.genesis.config.clone(), "hoodi"),
            Some(NamedChain::Holesky) => (HOLESKY.genesis.config.clone(), "holesky"),
            _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
        };

        // Fetch the execution witness
        let witness =
            DebugApiClient::<()>::debug_execution_witness(&client, BlockNumberOrTag::Number(block_number))
                .await
                .with_context(|| {
                    format!("Failed to fetch execution witness for block {}", block_number)
                })?;

        // Fetch the block
        let block = EthApiClient::<
            TransactionRequest,
            Transaction,
            Block<TransactionSigned>,
            Receipt,
            Header,
            TransactionSigned,
        >::block_by_number(&client, BlockNumberOrTag::Number(block_number), true)
        .await
        .with_context(|| format!("Failed to fetch block {}", block_number))?
        .with_context(|| format!("Block {} not found", block_number))?;

        // Get transaction count and gas used from the block
        let tx_count = block.transactions.len();
        let gas_used = block.header.gas_used;
        let mgas = gas_used / 1_000_000;

        // Create the fixture
        let fixture = StatelessValidationFixture {
            name: format!(
                "{}_{}_{}_{}_zec_reth",
                chain_name, block_number, tx_count, mgas
            ),
            stateless_input: StatelessInput {
                block: block.clone().into_consensus(),
                witness,
                chain_config: chain_config.clone(),
            },
            success: true,
        };

        // Generate the reth input
        let reth_input = StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
            .with_context(|| {
                format!(
                    "Failed to create StatelessValidatorReth input for {}",
                    fixture.name
                )
            })?;

        // Serialize the input to bytes
        let reth_input_bytes = bincode::serialize(&reth_input)?;

        let gas_used = block.header.gas_used;
        let tx_count = block.transactions.len() as u64;
        Ok(InputGeneratorResult {
            guest: GuestProgram::Reth,
            input: reth_input_bytes,
            gas_used,
            tx_count,
        })
    }

}

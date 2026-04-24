use anyhow::{Context, Result};
use async_trait::async_trait;
use zisk_sdk::ZiskStdin;

use guest_reth::{RethInput, RethInputPublic, RethInputWitness};
use stateless_reth::StatelessInput;

use super::{client::ExecutionClient, FromRpc};

#[derive(Default)]
pub struct RethClient;

impl RethClient {
    fn build_stdin(&self, input: &RethInput) -> Result<ZiskStdin> {
        let public = RethInputPublic {
            block: input.stateless_input.block.clone(),
            chain_config: input.stateless_input.chain_config.clone(),
            public_keys: input.public_keys.clone(),
        };
        let public_bytes =
            RethInputPublic::serialize(&public).context("Failed to serialize public input")?;
        let witness = RethInputWitness {
            witness: input.stateless_input.witness.clone(),
        };
        let witness_bytes =
            RethInputWitness::serialize(&witness).context("Failed to serialize witness")?;

        let stdin = ZiskStdin::new();
        stdin.write_slice(&public_bytes);
        stdin.write_slice(&witness_bytes);
        Ok(stdin)
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

    async fn from_rpc(&self, rpc_url: &str, block_number: u64) -> Result<ZiskStdin> {
        let input = RethInput::from_rpc(rpc_url, block_number)
            .await
            .with_context(|| format!("Failed to fetch RethInput for block {block_number}"))?;
        self.build_stdin(&input)
    }

    fn from_stateless_input(&self, stateless_input: &StatelessInput) -> Result<ZiskStdin> {
        let input = RethInput::new(stateless_input)
            .context("Failed to create RethInput from StatelessInput")?;
        self.build_stdin(&input)
    }

    fn run(&self) {
        guest_reth::run();
    }
}

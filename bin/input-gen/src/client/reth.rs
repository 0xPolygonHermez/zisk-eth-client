use anyhow::{Context, Result};

use stateless_validator_reth::guest::StatelessValidatorRethInput;
use witness_generator::StatelessValidationFixture;

use super::{ClientType, ExecutionClient};
use crate::processor::ProcessingResult;

pub struct RethClient;

impl RethClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RethClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionClient for RethClient {
    fn name(&self) -> &'static str {
        "reth"
    }

    fn client_type(&self) -> ClientType {
        ClientType::Reth
    }

    fn generate_input(&self, fixture: &StatelessValidationFixture) -> Result<ProcessingResult> {
        let reth_input =
            StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
                .with_context(|| {
                    format!(
                        "Failed to create StatelessValidatorReth input for {}",
                        fixture.name
                    )
                })?;
        Ok(ProcessingResult::Reth(reth_input))
    }
}

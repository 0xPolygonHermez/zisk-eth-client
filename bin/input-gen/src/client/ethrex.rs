use anyhow::{Context, Result};

use stateless_validator_ethrex::guest::StatelessValidatorEthrexInput;
use witness_generator::StatelessValidationFixture;

use super::{ClientType, ExecutionClient};
use crate::processor::ProcessingResult;

pub struct EthrexClient;

impl EthrexClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EthrexClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionClient for EthrexClient {
    fn name(&self) -> &'static str {
        "ethrex"
    }

    fn client_type(&self) -> ClientType {
        ClientType::Ethrex
    }

    fn generate_input(&self, fixture: &StatelessValidationFixture) -> Result<ProcessingResult> {
        let ethrex_input =
            StatelessValidatorEthrexInput::new(&fixture.stateless_input, fixture.success)
                .with_context(|| {
                    format!(
                        "Failed to create StatelessValidatorEthrex input for {}",
                        fixture.name
                    )
                })?;
        Ok(ProcessingResult::Ethrex(ethrex_input))
    }
}

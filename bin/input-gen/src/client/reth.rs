use anyhow::{Context, Result};

use witness_generator::StatelessValidationFixture;

use guest_reth::RethInput;

use super::ExecutionClient;
use crate::processor::ProcessingResult;

pub struct RethClient;

impl RethClient {
    pub fn new() -> Self {
        Self
    }
}

impl ExecutionClient for RethClient {
    fn name(&self) -> &'static str {
        "reth"
    }

    fn display_name(&self) -> &'static str {
        "Reth"
    }

    fn generate_input(&self, fixture: &StatelessValidationFixture) -> Result<ProcessingResult> {
        let reth_input = RethInput::new(&fixture.stateless_input)
            .with_context(|| format!("Failed to create {} input for {}", self.display_name(), fixture.name))?;
        Ok(ProcessingResult::Reth(reth_input))
    }
}

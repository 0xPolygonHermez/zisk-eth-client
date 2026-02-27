use clap::ValueEnum;

/// Execution client to generate inputs for
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ExecutionClient {
    /// Reth stateless block validation
    #[default]
    Reth,
    /// Ethrex stateless block validation
    Ethrex,
}

impl ExecutionClient {
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionClient::Reth => "reth",
            ExecutionClient::Ethrex => "ethrex",
        }
    }
}

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Binary format
    #[default]
    Binary,
    /// JSON format
    Json,
}

/// Execution client to generate inputs for
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ExecutionClient {
    /// Reth stateless block validation
    #[default]
    Reth,
    /// Ethrex stateless block validation
    Ethrex,
}

impl std::fmt::Display for ExecutionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionClient::Reth => write!(f, "reth"),
            ExecutionClient::Ethrex => write!(f, "ethrex"),
        }
    }
}

impl ExecutionClient {
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionClient::Reth => "reth",
            ExecutionClient::Ethrex => "ethrex",
        }
    }
}

use clap::ValueEnum;

#[derive(Debug, Clone, Default)]
pub struct InputResult {
    pub input: Vec<u8>,
    pub gas_used: u64,
    pub tx_count: u64,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Binary format
    #[default]
    Binary,
    /// JSON format
    Json,
}

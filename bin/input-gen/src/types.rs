use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Binary format
    #[default]
    Binary,
    /// JSON format
    Json,
}

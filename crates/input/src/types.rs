use std::fmt::Display;

use async_trait::async_trait;
use clap::ValueEnum;
use url::Url;

#[derive(Debug, Clone, ValueEnum)]
pub enum Network {
    Mainnet,
    Sepolia,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum GuestProgram {
    Rsp,
    Zeth,
}

pub struct InputGeneratorConfig {
    pub guest: GuestProgram,
    pub rpc_url: Url,
    pub network: Option<Network>,
}

pub struct InputGeneratorResult {
    pub input: Vec<u8>,
    pub gas_used: u64,
    pub tx_count: u64,
}

impl Display for GuestProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuestProgram::Rsp => write!(f, "rsp"),
            GuestProgram::Zeth => write!(f, "zeth"),
        }
    }
}

#[async_trait]
pub trait InputGenerator {
    async fn generate(&self, block_number: u64) -> anyhow::Result<InputGeneratorResult>;

    fn get_config(&self) -> &InputGeneratorConfig;
}

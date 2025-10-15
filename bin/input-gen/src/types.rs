use std::{fmt::Display, io::Write, path::PathBuf};

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
    pub input_dir: Option<std::path::PathBuf>,
}

pub struct InputGeneratorResult {
    pub input_file_path: PathBuf,
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

    fn save_input_to_file (&self, block_number: u64, gas_used: u64, txcount: u64, input: Vec<u8>) -> anyhow::Result<PathBuf> {
        let config = self.get_config();

        // Create the input directory if it does not exist.
        let input_folder = config.input_dir.clone().unwrap_or("inputs".into());
        if !input_folder.exists() {
            std::fs::create_dir_all(&input_folder)?;
        }

        // Save the input to a file
        let mgas = (gas_used + 999_999) / 1_000_000;
        let input_path = input_folder.join(format!(
            "{}_{}_{}_{}.bin",
            block_number,
            txcount,
            mgas,
            config.guest
        ));
        let mut input_file = std::fs::File::create(&input_path)?;
        input_file.write_all(&input)?;

        Ok(input_path)
    }

    fn get_config(&self) -> &InputGeneratorConfig;
}

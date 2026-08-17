use input_core::ExecutionClient;

/// Execution clients. To add one, see `docs/adding-a-client.md` — adding a
/// variant here makes `create_client` (and the host's ELF/input-gen matches)
/// fail to compile until you wire up the new client.
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    #[cfg(feature = "reth")]
    Reth,
    #[cfg(feature = "ethrex")]
    Ethrex,
    #[cfg(feature = "ziskethone")]
    #[cfg_attr(feature = "cli", value(name = "ziskethone"))]
    ZiskEthOne,
}

pub fn create_client(client: Client) -> Box<dyn ExecutionClient> {
    match client {
        #[cfg(feature = "reth")]
        Client::Reth => Box::new(input_reth::RethClient),
        #[cfg(feature = "ethrex")]
        Client::Ethrex => Box::new(input_ethrex::EthrexClient),
        #[cfg(feature = "ziskethone")]
        Client::ZiskEthOne => Box::new(input_ziskethone::ZiskEthOneClient),
    }
}

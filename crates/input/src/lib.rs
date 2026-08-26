mod client;

pub use client::{create_client, Client};
pub use input_core::{
    generate_hints_to_file, generate_hints_to_socket, parse_header, BlockStats, ExecutionClient,
    InputStats, RpcConfig,
};

#[cfg(feature = "ethrex")]
pub use input_ethrex::EthrexClient;
#[cfg(feature = "reth")]
pub use input_reth::RethClient;
#[cfg(feature = "ziskethone")]
pub use input_ziskethone::ZiskEthOneClient;

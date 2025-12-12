mod rsp;
mod types;
mod zeth;

pub use types::{
    GuestProgram, InputGenerator, InputGeneratorConfig, InputGeneratorResult, Network,
};

use url::Url;

use crate::{rsp::RspInputGenerator, zeth::ZethInputGenerator};

// Re-export the important types so other workspace crates can use them.

pub fn build_input_generator(
    guest: GuestProgram,
    rpc_url: &str,
    network: Network,
) -> Box<dyn InputGenerator> {
    let config = InputGeneratorConfig {
        guest: guest.clone(),
        rpc_url: Url::parse(rpc_url).expect("Invalid RPC URL"),
        network,
    };

    match guest {
        GuestProgram::Rsp => Box::new(RspInputGenerator::new(config)),
        GuestProgram::Zeth => Box::new(ZethInputGenerator::new(config)),
    }
}

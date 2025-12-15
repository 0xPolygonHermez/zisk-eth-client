mod types;
#[cfg(feature = "zec-rsp")]
mod rsp;
#[cfg(not(feature = "zec-rsp"))]
mod zeth;

pub use types::{InputGenerator, InputGeneratorResult, Network};

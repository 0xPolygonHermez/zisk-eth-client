#[cfg(feature = "zec-rsp")]
mod rsp;
mod types;
#[cfg(not(feature = "zec-rsp"))]
mod zeth;

pub use types::{InputGenerator, InputGeneratorResult, Network};

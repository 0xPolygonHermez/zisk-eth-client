#[cfg(feature = "zec-rsp")]
mod rsp;
#[cfg(feature = "zec-zeth")]
mod zeth;
#[cfg(not(any(feature = "zec-rsp", feature = "zec-zeth")))]
mod reth;

mod types;

pub use types::{InputGenerator, InputGeneratorResult, Network};

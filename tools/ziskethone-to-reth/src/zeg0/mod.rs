pub mod reader;
pub mod trie;

pub use reader::parse;
pub use trie::{check_root, rebuild};

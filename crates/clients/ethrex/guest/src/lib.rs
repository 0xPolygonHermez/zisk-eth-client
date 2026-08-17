use rkyv::{rancor::Error, Archive, Deserialize as RDeserliaze, Serialize as RSerialize};
use serde::{Deserialize, Serialize};

use ethrex_common::types::{block_execution_witness::ExecutionWitness, Block};

mod crypto;
mod run;
mod utils;
mod validation;

pub use crypto::*;
pub use run::*;
pub use utils::*;
pub use validation::*;

#[derive(Clone, Serialize, Deserialize, RSerialize, RDeserliaze, Archive)]
pub struct EthrexInput {
    /// Block to execute
    pub block: Block,
    /// Database containing all the data necessary to execute
    pub execution_witness: ExecutionWitness,
}

impl EthrexInput {
    pub fn new(block: Block, execution_witness: ExecutionWitness) -> Self {
        Self {
            block,
            execution_witness,
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn witness(&self) -> &ExecutionWitness {
        &self.execution_witness
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        rkyv::to_bytes(self).map(|vec| vec.to_vec())
    }

    /// Deserialize from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        rkyv::from_bytes(bytes)
    }
}

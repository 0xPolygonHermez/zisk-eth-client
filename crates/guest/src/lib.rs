use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use alloy_genesis::ChainConfig;
use alloy_rpc_types_debug::ExecutionWitness;

use reth_ethereum_primitives::{Block, TransactionSigned};
use stateless_reth::{StatelessInput, UncompressedPublicKey};

mod utils;
mod validation;

pub use utils::*;
pub use validation::*;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInput {
    /// The stateless input for the stateless validation function.
    pub stateless_input: StatelessInput,
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

impl RethInput {
    pub fn new(stateless_input: &StatelessInput) -> anyhow::Result<Self> {
        let signers = recover_signers(&stateless_input.block.body.transactions)?;

        Ok(Self {
            stateless_input: stateless_input.clone(),
            public_keys: signers,
        })
    }
}

/// Wrapper for witness part (StatelessInput without public keys)
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInputWitness {
    /// The stateless input (block, witness, chain_config)
    pub stateless_input: StatelessInput,
}

impl RethInputWitness {
    /// Get the block
    pub fn block(&self) -> &Block {
        &self.stateless_input.block
    }

    /// Get the execution witness
    pub fn witness(&self) -> &ExecutionWitness {
        &self.stateless_input.witness
    }

    /// Get the chain config
    pub fn chain_config(&self) -> &ChainConfig {
        &self.stateless_input.chain_config
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize witness")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize witness")
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RethInputPublic {
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

impl RethInputPublic {
    /// Get the public keys
    pub fn public_keys(&self) -> &Vec<UncompressedPublicKey> {
        &self.public_keys
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self.public_keys).context("Failed to serialize public keys")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Failed to deserialize public keys")
    }
}

// Recovers the signing [`UncompressedPublicKey`] from each transaction's signature, in parallel.
pub fn recover_signers(txs: &[TransactionSigned]) -> Result<Vec<UncompressedPublicKey>> {
    txs.par_iter()
        .enumerate()
        .map(|(i, tx)| {
            let keys = tx
                .signature()
                .recover_from_prehash(&tx.signature_hash())
                .with_context(|| format!("Failed to recover signature for tx #{i}"))?;

            let encoded_point: [u8; 65] = keys
                .to_encoded_point(false)
                .as_bytes()
                .try_into()
                .map_err(|e| anyhow!("Failed to encode public key for tx #{i}, error: {e}"))?;

            Ok(UncompressedPublicKey(encoded_point))
        })
        .collect()
}

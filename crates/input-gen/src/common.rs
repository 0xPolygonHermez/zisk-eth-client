//! EEST fixture loading.
//!
//! Since tests-zkevm v21 every `blockchain_test` block and every
//! `blockchain_test_engine` payload carries the stateless guest input bytes
//! (`statelessInputBytes`) and the expected guest output
//! (`statelessOutputBytes`), so a fixture needs no re-execution to become a
//! guest input: the bytes are used verbatim.

use std::collections::BTreeMap;
use std::path::Path;

use alloy_primitives::Bytes;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use walkdir::WalkDir;

/// One block of an EEST fixture, as the guest input and expected output bytes.
#[derive(Debug, Clone)]
pub struct StatelessFixture {
    /// `<test id>#block<n>` or `<test id>#payload<n>`.
    pub name: String,
    /// Schema-prefixed SSZ `StatelessInput`.
    pub stateless_input_bytes: Vec<u8>,
    /// Plain SSZ `StatelessValidationResult` the guest must produce.
    pub stateless_output_bytes: Vec<u8>,
}

/// Reads every fixture JSON file under `path`, sorted by name.
pub fn fixtures_from_path(path: &Path) -> Result<Vec<StatelessFixture>> {
    let mut fixtures = WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
        })
        .par_bridge()
        .map(|entry| fixtures_from_file(entry.path()))
        .try_reduce(Vec::new, |mut acc, mut more| {
            acc.append(&mut more);
            Ok(acc)
        })?;
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

fn fixtures_from_file(path: &Path) -> Result<Vec<StatelessFixture>> {
    let content = std::fs::read(path)?;
    let tests: BTreeMap<String, EestTest> = serde_json::from_slice(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(tests
        .into_iter()
        .flat_map(|(test_id, test)| {
            let blocks = test.blocks.into_iter().map(|b| ("block", b));
            let payloads = test.engine_new_payloads.into_iter().map(|b| ("payload", b));
            blocks
                .chain(payloads)
                .enumerate()
                .filter_map(move |(idx, (label, block))| {
                    let (input, output) = block
                        .stateless_input_bytes
                        .zip(block.stateless_output_bytes)?;
                    (!input.is_empty()).then(|| StatelessFixture {
                        name: format!("{test_id}#{label}{idx}"),
                        stateless_input_bytes: input.to_vec(),
                        stateless_output_bytes: output.to_vec(),
                    })
                })
        })
        .collect())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestTest {
    #[serde(default)]
    blocks: Vec<EestBlock>,
    #[serde(default)]
    engine_new_payloads: Vec<EestBlock>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlock {
    stateless_input_bytes: Option<Bytes>,
    stateless_output_bytes: Option<Bytes>,
}

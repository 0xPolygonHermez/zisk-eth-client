//! Shared helpers for guest programs that drive precompile / EVM tests:
//! JSON-fixture parsing (success and failure cases), and a small hex utility.

use serde::Deserialize;

#[derive(Clone)]
pub enum ExpectedOutcome {
    Success(Vec<u8>),
    Failure(String),
}

impl ExpectedOutcome {
    pub fn unwrap_success(&self) -> &Vec<u8> {
        match self {
            ExpectedOutcome::Success(bytes) => bytes,
            ExpectedOutcome::Failure(e) => panic!("expected Success, got Failure: {e}"),
        }
    }
}

pub struct PrecompileTestCase {
    pub name: String,
    pub input: Vec<u8>,
    pub expected: ExpectedOutcome,
}

#[derive(Deserialize)]
struct SuccessJsonTest {
    #[serde(rename = "Input")]
    input: String,
    #[serde(rename = "Expected")]
    expected: String,
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Deserialize)]
struct FailJsonTest {
    #[serde(rename = "Input")]
    input: String,
    #[serde(rename = "ExpectedError")]
    expected_error: String,
    #[serde(rename = "Name")]
    name: String,
}

pub fn parse_precompile_json(json_content: &str) -> Vec<PrecompileTestCase> {
    let tests: Vec<SuccessJsonTest> = serde_json::from_str(json_content).expect("valid JSON");
    tests
        .into_iter()
        .map(|t| PrecompileTestCase {
            name: t.name,
            input: hex::decode(&t.input).expect("valid hex"),
            expected: ExpectedOutcome::Success(hex::decode(&t.expected).expect("valid hex")),
        })
        .collect()
}

pub fn parse_precompile_fail_json(json_content: &str) -> Vec<PrecompileTestCase> {
    let tests: Vec<FailJsonTest> = serde_json::from_str(json_content).expect("valid JSON");
    tests
        .into_iter()
        .map(|t| PrecompileTestCase {
            name: t.name,
            input: hex::decode(&t.input).expect("valid hex"),
            expected: ExpectedOutcome::Failure(t.expected_error),
        })
        .collect()
}

pub fn hex_to_vec(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("valid hex")
}

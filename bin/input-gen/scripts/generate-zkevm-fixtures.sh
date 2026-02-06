#!/usr/bin/env bash

# TODO: I should do this via importing the crate!

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKLOAD_DIR="${SCRIPT_DIR}/../zkevm-benchmark-workload"
REPO_URL="https://github.com/eth-act/zkevm-benchmark-workload.git"
OUTPUT_DIR="${SCRIPT_DIR}/../zkevm-fixtures"

export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-10}"
export RUST_LOG="${RUST_LOG:-info}"

# Check for --help flag
for arg in "$@"; do
    if [[ "$arg" == "--help" || "$arg" == "-h" ]]; then
        # Ensure workload dir exists for help
        if [ ! -d "${WORKLOAD_DIR}" ]; then
            git clone --depth 1 "${REPO_URL}" "${WORKLOAD_DIR}"
        fi
        cd "${WORKLOAD_DIR}"
        cargo run --release --bin witness-generator-cli -- "$@"
        exit 0
    fi
done

if [ ! -d "${WORKLOAD_DIR}" ]; then
    echo "Cloning zkevm-benchmark-workload..."
    git clone --depth 1 "${REPO_URL}" "${WORKLOAD_DIR}"
fi

# Generate fixtures
echo "Generating fixtures..."

cd "${WORKLOAD_DIR}"
cargo run --release --bin witness-generator-cli -- -o "${OUTPUT_DIR}" "$@"

echo "Fixtures generated at: ${OUTPUT_DIR}"
#!/usr/bin/env bash

# TODO: I should do this via importing the crate!

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKLOAD_DIR="${SCRIPT_DIR}/../zkevm-benchmark-workload"
REPO_URL="https://github.com/eth-act/zkevm-benchmark-workload.git"
CLEANUP=${CLEANUP:-false}

if [ ! -d "${WORKLOAD_DIR}" ]; then
    echo "Cloning zkevm-benchmark-workload..."
    git clone --depth 1 "${REPO_URL}" "${WORKLOAD_DIR}"
fi

cd "${WORKLOAD_DIR}"
RUST_LOG=info RAYON_NUM_THREADS=10 cargo run --release --bin witness-generator-cli -- -o ../zkevm-fixtures-input "$@"

if [ "${CLEANUP}" = "true" ]; then
    echo "Cleaning up zkevm-benchmark-workload..."
    rm -rf "${WORKLOAD_DIR}"
fi
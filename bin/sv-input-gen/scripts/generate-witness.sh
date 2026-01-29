#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKLOAD_DIR="${SCRIPT_DIR}/../zkevm-benchmark-workload"

cd "${WORKLOAD_DIR}"
RUST_LOG=info RAYON_NUM_THREADS=10 cargo run --release --bin witness-generator-cli -- -o ../zkevm-fixtures-input "$@"
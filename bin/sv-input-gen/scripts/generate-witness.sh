#!/usr/bin/env bash

# TODO: I should do this via importing the crate!

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKLOAD_DIR="${SCRIPT_DIR}/../zkevm-benchmark-workload"
REPO_URL="https://github.com/eth-act/zkevm-benchmark-workload.git"
OUTPUT_DIR="${SCRIPT_DIR}/../zkevm-fixtures-input"

export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-10}"
export RUST_LOG="${RUST_LOG:-info}"

if [ ! -d "${WORKLOAD_DIR}" ]; then
    echo "Cloning zkevm-benchmark-workload..."
    git clone --depth 1 "${REPO_URL}" "${WORKLOAD_DIR}"
fi

# Generate fixtures
echo "Generating fixtures..."
cd "${WORKLOAD_DIR}"
cargo run --release --bin witness-generator-cli -- -o "${OUTPUT_DIR}" "$@"

# Organize fixtures by gas category
echo "Organizing fixtures by gas..."

cd "${OUTPUT_DIR}"

# Process each json file
for file in *.json; do
    [ -e "$file" ] || continue
    
    # Try to extract gas value from filename (e.g., gas-value_100M, gas-value_10M, etc.)
    if [[ "$file" =~ gas-value_([0-9]+[KMG]?) ]]; then
        gas_category="${BASH_REMATCH[1]}"
        target_dir="${gas_category}"
    else
        # No gas value found, put in 'uncategorized' folder
        target_dir="uncategorized"
    fi
    
    # Create directory if it doesn't exist
    mkdir -p "${target_dir}"
    
    # Move file to appropriate directory
    mv "$file" "${target_dir}/"
done

echo "Fixtures organized into gas categories:"
for dir in $(ls -d */ 2>/dev/null | sed 's/\///' | sort -t'M' -k1 -n); do
    [ -d "$dir" ] || continue
    count=$(find "$dir" -maxdepth 1 -name "*.json" | wc -l)
    echo "  ${dir}: ${count} files"
done

# Cleanup
echo "Cleaning up..."
rm -rf "${WORKLOAD_DIR}"
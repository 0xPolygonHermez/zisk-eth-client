#!/bin/bash
# Migrate old ziskethone inputs (ZEG0 v8/v9) to the current ZEG0 format.
# Offline: no RPC needed.
#
# Goes through a reth input: ZEG0 -> reth .bin (ziskethone-to-reth --offline)
# -> ZEG0 (reth-to-ziskethone --fill-outputs). The second step executes the
# block to fill the header fields the old file lacks, validates it with reth,
# and records the accessed keys that ZEG0 v12+ requires.
#
# Inputs named <block>_<hash prefix>.bin are also checked against that hash.
#
# Files are converted one by one (JOBS in parallel). Converted inputs are
# listed in <output_dir>/.migrated, so a rerun skips them; failures go to
# <output_dir>/.failed.
#
# Usage: ./migrate-ziskethone-input.sh <input .bin | dir> [output_dir]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${CARGO_TARGET_DIR:-$ROOT/target}/release"
TO_RETH="$BIN_DIR/ziskethone-to-reth"
TO_ZEG="$BIN_DIR/reth-to-ziskethone"

# Worker mode: convert a single file. Called through xargs below.
if [[ "${1:-}" == "--one" ]]; then
    IN="$2" OUTPUT_DIR="$3"
    NAME="$(basename "$IN")"
    TMP="$(mktemp -d)"
    trap 'rm -rf "$TMP"' EXIT
    fail() {
        echo "$NAME" >> "$OUTPUT_DIR/.failed"
        echo "FAIL $NAME: $1"
        exit 0
    }
    mkdir "$TMP/reth" "$TMP/out"
    "$TO_RETH" "$IN" --offline -o "$TMP/reth" > "$TMP/log" 2>&1 \
        && "$TO_ZEG" "$TMP/reth" --fill-outputs -o "$TMP/out" >> "$TMP/log" 2>&1 \
        || fail "$(grep -m1 -E '^err|Error' "$TMP/log")"
    HASH="$(sed -n 's/^filled block [0-9]* hash 0x//p' "$TMP/log")"
    if [[ "$NAME" =~ ^[0-9]+_([0-9a-f]+)\.bin$ && "$HASH" != "${BASH_REMATCH[1]}"* ]]; then
        fail "block hash 0x$HASH does not match the file name"
    fi
    mv "$TMP"/out/*.bin "$OUTPUT_DIR/"
    echo "$NAME" >> "$OUTPUT_DIR/.migrated"
    echo "ok   $NAME"
    exit 0
fi

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <input .bin | dir> [output_dir]"
    echo "  output_dir: where to write *_zec_ziskethone.bin (default: ziskethone-inputs)"
    echo "  JOBS:       parallel conversions (default: 8)"
    exit 1
fi

INPUT="$1"
OUTPUT_DIR="${2:-$ROOT/ziskethone-inputs}"
JOBS="${JOBS:-8}"

echo "Building transcoders..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p ziskethone-to-reth -p reth-to-ziskethone

mkdir -p "$OUTPUT_DIR"
touch "$OUTPUT_DIR/.migrated"
rm -f "$OUTPUT_DIR/.failed"

if [[ -d "$INPUT" ]]; then
    find "$INPUT" -maxdepth 1 -name '*.bin' | sort
else
    echo "$INPUT"
fi | while read -r f; do grep -qxF "$(basename "$f")" "$OUTPUT_DIR/.migrated" || echo "$f"; done \
   | xargs -r -P "$JOBS" -I{} "$0" --one {} "$OUTPUT_DIR"

touch "$OUTPUT_DIR/.failed"
echo "Done: $(wc -l < "$OUTPUT_DIR/.migrated") migrated, $(wc -l < "$OUTPUT_DIR/.failed") failed"

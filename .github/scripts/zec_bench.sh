#!/usr/bin/env bash
#
# zec_bench.sh — build the stateless-validator guest ELFs from the *current*
# working tree and emulate each one with `ziskemu -X` over a fixed set of block
# inputs, dumping the per-(client, block) REPORT into <OUTDIR>/<client>_<block>.txt.
#
# It is run twice by the cycle-tracking workflow: once on the PR tree and once
# on the base-branch tree. Building from the current tree each time means the
# diff reflects changes to the guest sources / resulting ELF (the workflow holds
# the emulator fixed across both runs).
#
# Usage: zec_bench.sh <OUTDIR>
#
# Requirements (provided by the workflow before calling this):
#   - CARGO_ZISK / ZISKEMU env vars pointing at the binaries built from the local
#     ZisK clone, and the ZisK rust toolchain installed
#   - RETH_BLOCKS / ETHREX_BLOCKS env vars (space-separated block numbers)
set -euo pipefail

OUTDIR="${1:?usage: zec_bench.sh <OUTDIR>}"
mkdir -p "$OUTDIR"

CARGO_ZISK="${CARGO_ZISK:-cargo-zisk}"
ZISKEMU="${ZISKEMU:-ziskemu}"

REPO="${BENCH_REPO_DIR:-${GITHUB_WORKSPACE:-$(git rev-parse --show-toplevel)}}"
cd "$REPO"

ZISK_TARGET="riscv64ima-zisk-zkvm-elf"

# client -> space-separated block numbers to emulate.
declare -A CLIENT_BLOCKS=(
  [reth]="${RETH_BLOCKS:-25229957}"
  [ethrex]="${ETHREX_BLOCKS:-25229957}"
)

for client in reth ethrex; do
  guest_dir="bin/guests/stateless-validator-${client}"

  echo "::group::Build ${client} guest ELF"
  # Build from the guest's own manifest, the same way the host's build.rs does.
  # Tolerate failure: a client/guest that is new in this PR won't build on the
  # base pass, and the diff renders the missing base side as N/A.
  if ! ( cd "$guest_dir" && "$CARGO_ZISK" build --release ); then
    echo "WARNING: build failed for '$client' guest; skipping" >&2
    echo "::endgroup::"
    continue
  fi
  echo "::endgroup::"

  elf="$guest_dir/target/elf/$ZISK_TARGET/release/zec-${client}"
  if [[ ! -f "$elf" ]]; then
    echo "WARNING: build reported success but no ELF for '$client' at $elf; skipping" >&2
    continue
  fi

  for block in ${CLIENT_BLOCKS[$client]}; do
    input=$(ls "$guest_dir"/inputs/mainnet_"${block}"_*.bin 2>/dev/null | head -n1 || true)
    if [[ -z "$input" || ! -f "$input" ]]; then
      echo "WARNING: no input file for ${client} block ${block}; skipping" >&2
      continue
    fi
    echo "::group::Emulate ${client} ${block}"
    # -X prints the REPORT SUMMARY (STEPS + COST DISTRIBUTION) to stdout.
    "$ZISKEMU" -e "$elf" -i "$input" -X | tee "$OUTDIR/${client}_${block}.txt"
    echo "::endgroup::"
  done
done

echo "Reports written to $OUTDIR:"
ls -1 "$OUTDIR"

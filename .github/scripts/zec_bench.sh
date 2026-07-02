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
#   - RETH_TEST_BLOCKS / ETHREX_TEST_BLOCKS / ZISKETHONE_TEST_BLOCKS env vars
#     (space-separated block numbers)
#   - for ziskethone (Pattern B): the third_party/ziskethone submodule and the
#     C++ guest cross-toolchain (build-elf.sh cross-compiles its ELF)
set -euo pipefail

OUTDIR="${1:?usage: zec_bench.sh <OUTDIR>}"
mkdir -p "$OUTDIR"

CARGO_ZISK="${CARGO_ZISK:-cargo-zisk}"
ZISKEMU="${ZISKEMU:-ziskemu}"

REPO="${BENCH_REPO_DIR:-${GITHUB_WORKSPACE:-$(git rev-parse --show-toplevel)}}"
cd "$REPO"

ZISK_TARGET="riscv64ima-zisk-zkvm-elf"

# client -> space-separated block numbers to emulate. All clients should use the
# same blocks so the report can compare them against each other (reth baseline).
declare -A CLIENT_BLOCKS=(
  [reth]="${RETH_TEST_BLOCKS:-25229957}"
  [ethrex]="${ETHREX_TEST_BLOCKS:-25229957}"
  [ziskethone]="${ZISKETHONE_TEST_BLOCKS:-25229957}"
)

# Build a client's guest ELF and echo its path on success (nothing on failure).
# reth/ethrex are Pattern A (cargo-zisk builds the Rust guest). ziskethone is
# Pattern B: the C++ guest ELF is cross-compiled by build-elf.sh and lives in the
# third_party/ziskethone submodule — there is no cargo-zisk guest crate for it.
build_guest_elf() {
  local client="$1"
  if [[ "$client" == "ziskethone" ]]; then
    local script="crates/guest-ziskethone/build-elf.sh"
    [[ -f "$script" ]] || { echo "WARNING: $script not found (submodule absent?); skipping ziskethone" >&2; return 1; }
    bash "$script" >&2 || return 1
    # build-elf.sh writes to <ziskethone>/cpp-guest/zisk/build/zisk_eth_guest.elf;
    # the default ziskethone dir is the third_party/ziskethone submodule.
    local elf="${ZISKETHONE_DIR:-third_party/ziskethone}/cpp-guest/zisk/build/zisk_eth_guest.elf"
    [[ -f "$elf" ]] && { echo "$elf"; return 0; }
    echo "WARNING: ziskethone build reported success but no ELF at $elf" >&2
    return 1
  fi

  local guest_dir="bin/guests/stateless-validator-${client}"
  ( cd "$guest_dir" && "$CARGO_ZISK" build --release ) >&2 || return 1
  local elf="$guest_dir/target/elf/$ZISK_TARGET/release/zec-${client}"
  [[ -f "$elf" ]] && { echo "$elf"; return 0; }
  echo "WARNING: $client build reported success but no ELF at $elf" >&2
  return 1
}

for client in reth ethrex ziskethone; do
  guest_dir="bin/guests/stateless-validator-${client}"

  echo "::group::Build ${client} guest ELF"
  # Tolerate failure: a client/guest that is new in this PR won't build on the
  # base pass, and the diff renders the missing base side as N/A.
  if ! elf="$(build_guest_elf "$client")"; then
    echo "WARNING: build failed for '$client' guest; skipping" >&2
    echo "::endgroup::"
    continue
  fi
  echo "::endgroup::"

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

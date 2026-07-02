#!/usr/bin/env bash
# Run the host stateless-validator over the configured test blocks for one client.
#
# Usage: tools/ci/run-validation.sh <client> <action> <executor> [mode]
#   client   : reth | ethrex | ziskethone
#   action   : execute | verify-constraints
#   executor : emulator | assembly
#   mode     : standard (default) | hints
#
# In `hints` mode the run adds `--gen-hints` (generate hints from the input, then
# run with them). Hints require the assembly executor, and the host must have been
# built with RUSTFLAGS="--cfg zisk_hints" — otherwise host rejects --gen-hints.
# The ziskethone client is skipped in hints mode (CI does not run it there).
#
# Environment:
#   <CLIENT>_TEST_BLOCKS  (required) space-separated block numbers to --include,
#                         per client (e.g. RETH_TEST_BLOCKS, ETHREX_TEST_BLOCKS).
#                         TEST_BLOCKS may be set instead to override for all clients.
#   EXTRA_FLAGS           (optional) extra host flags, e.g. "--gpu --proving-key /path"
set -euo pipefail

client="${1:?usage: run-validation.sh <client> <action> <executor> [mode]}"
action="${2:?usage: run-validation.sh <client> <action> <executor> [mode]}"
executor="${3:?usage: run-validation.sh <client> <action> <executor> [mode]}"
mode="${4:-standard}"

# Resolve the per-client block list (e.g. RETH_TEST_BLOCKS). A global TEST_BLOCKS,
# if set, overrides the per-client variable.
client_blocks_var="$(echo "$client" | tr '[:lower:]' '[:upper:]')_TEST_BLOCKS"
TEST_BLOCKS="${TEST_BLOCKS:-${!client_blocks_var:-}}"
: "${TEST_BLOCKS:?set ${client_blocks_var} (or TEST_BLOCKS) to space-separated block numbers}"

input_folder="bin/guests/stateless-validator-${client}/inputs"

case "$executor" in
  emulator) executor_flag=(--emulator) ;;
  assembly) executor_flag=(--unlock-mapped-memory) ;;
  *) echo "ERROR: unknown executor '$executor' (expected emulator|assembly)" >&2; exit 1 ;;
esac

gen_hints_flag=()
case "$mode" in
  standard) ;;
  hints)
    [[ "$executor" == "assembly" ]] || {
      echo "ERROR: hints mode requires the assembly executor (host rejects --emulator with hints)" >&2
      exit 1
    }
    gen_hints_flag=(--gen-hints)
    ;;
  *) echo "ERROR: unknown mode '$mode' (expected standard|hints)" >&2; exit 1 ;;
esac

# Verify each block has an input file and build the --include flags.
shopt -s nullglob
include_flags=()
for block in $TEST_BLOCKS; do
  matches=("${input_folder}"/mainnet_"${block}"_*.bin)
  ((${#matches[@]} > 0)) || { echo "ERROR: no input file for block $block in $input_folder" >&2; exit 1; }
  include_flags+=(--include "$block")
done

# EXTRA_FLAGS is intentionally unquoted so multi-flag values word-split.
set -x
./target/release/host "${executor_flag[@]}" --action "$action" ${EXTRA_FLAGS:-} \
  stateless-validator --client "$client" --input-folder "$input_folder" \
  "${include_flags[@]}" "${gen_hints_flag[@]}"

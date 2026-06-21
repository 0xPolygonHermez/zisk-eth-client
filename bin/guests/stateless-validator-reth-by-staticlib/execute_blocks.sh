#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Build all flavours.
./build.sh 1   # host — with zisk_hints
./build.sh 2   # host — no hints
./build.sh 3   # ZisK zkVM
./build.sh 4   # bare RISC-V

HOST_ELF="$SCRIPT_DIR/target/release/zec-reth"
RISCV_ELF="$SCRIPT_DIR/target/riscv64im-unknown-none-elf/release/zec-reth"
ZISK_ELF="$SCRIPT_DIR/target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth"

echo "Execution for host"

for x in inputs/*; do
  ln -sf "$(realpath "$x")" build/input.bin
  # Exit code 61 is the ZisK host runtime's normal exit via zkvm_deinit(); treat it as success.
  "$HOST_ELF" || [ $? -eq 61 ]
done

export PATH=/projects/EF/zisk-repos/zisk/target/release:$PATH

echo "Execution for RISCV64IM"

for x in inputs/*; do
  ziskemu -e "$RISCV_ELF" -i "$x"
done

echo "Execution for ZisK"

for x in inputs/*; do
  ziskemu -e "$ZISK_ELF" -i "$x"
done

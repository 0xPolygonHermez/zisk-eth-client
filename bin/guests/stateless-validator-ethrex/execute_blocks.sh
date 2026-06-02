#!/bin/bash

cargo-zisk build --release
cargo build --release

for x in inputs/*; do
  ln -sf $(realpath $x) build/input.bin
  cargo run --release
done

export PATH=/projects/EF/zisk-repos/zisk/target/release:$PATH

for x in inputs/*; do
  ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/zec-ethrex -i $x;
done


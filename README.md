# ZISK ETH BLOCK

## Description

```bash
cargo-zisk run --release -m -s
```

In orther to change the block number, you can copy a block_number.bin from data into build/input.bin.

the block 18884865 has too many steps so you may see this error:
```bash
Error during emulation: EmulationNoCompleted
Error: Error executing Run command

Caused by:
    Cargo run command failed with status exit status: 1
```
So you need to run the ziskemu with more steps:
```bash
ziskemu -i build/input.bin -x -m -e target/riscv64ima-polygon-ziskos-elf/release/zisk_eth_block -n 10000000000
```

You can enable the the keccak precompile on the Cargo.toml file removing the comment to this line:
```bash
[patch.crates-io]
tiny-keccak = { git = "https://github.com/0xPolygonHermez/tiny-keccak", branch = "edu/zisk" }

```

in order to generate other blocks you can use [rsp](https://github.com/succinctlabs/rsp) tool directly:
```bash
rsp --block-number 18884864 --cache-dir ./data --rpc-url  RPC_URL 
```
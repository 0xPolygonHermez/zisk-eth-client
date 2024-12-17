## Zisk Ethereum Client

### Description
In the `data` folder, you will find example input files for Ethereum blocks. To run a specific block in the Zisk emulator, copy the `<block_number>.bin` file into the `build/input.bin` directory and execute one of the following commands:
```bash
cargo-zisk run --release -m -s
```
or 
```bash
ziskemu -i build/input.bin -x -m -e target/riscv64ima-polygon-ziskos-elf/release/zisk-eth-client
```

#### Note:
Block `18884865` contains too many steps, which may cause the following error:
```bash
Error during emulation: EmulationNoCompleted
Error: Error executing Run command

Caused by:
    Cargo run command failed with status exit status: 1
```
To resolve this, increase the number of steps (`-n`) when running `ziskemu`:
```bash
ziskemu -i build/input.bin -x -m -e target/riscv64ima-polygon-ziskos-elf/release/zisk-eth-block -n 10000000000
```

You can also enable the keccak precompile by uncommenting the following line in the `Cargo.toml` file:
```bash
[patch.crates-io]
tiny-keccak = { git = "https://github.com/0xPolygonHermez/tiny-keccak", branch = "edu/zisk" }

```
### Generating input block files
To generate additional input block files, you can use the [rsp](https://github.com/succinctlabs/rsp) tool:
```bash
rsp --block-number 18884864 --cache-dir ./data --rpc-url <RPC_URL>
```
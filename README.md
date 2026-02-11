# Zisk Ethereum Client

An experimental Ethereum execution client built for the ZisK zkVM.
It allows you to build, run, and test Ethereum block execution inside the ZisK emulator.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended).
- [cargo-zisk](https://0xpolygonhermez.github.io/zisk/getting_started/installation.html) (ZisK’s Cargo wrapper).
- A working Ethereum RPC endpoint (e.g. Infura, Alchemy, or your own node) for input files generation.

## Build the guest program (zec-reth)

```bash
cd bin/guest
cargo-zisk build --release
```

The compiled ELF file will be generated at:

```bash
./target/riscv64ima-zisk-zkvm-elf/release/zec-reth
```

### Execute Ethereum Blocks

Sample input files for Ethereum blocks are provided in the `inputs` folder

To run a block in the ZisK emulator, use:

```bash
cargo-zisk run --release -i ./inputs/23583300_208_18_mainnet_24341035_74_5_zec_reth.bin
```

Or, directly via the `ziskemu` tool:

```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/zec-reth -i ./inputs/mainnet_24341035_74_5_zec_reth.bin
```

## Generate Input Block Files

To generate your own input files, you can use the `input-gen` tool.

Example, generate an input file for block `22767493`:

```bash
cargo build --release
target/release/input-gen rpc --block 22767493 -u <RPC_URL>
```

Replace `<RPC_URL>` with the URL of an Ethereum Mainnet RPC endpoint.

The command will create a file named `cccccc_22767493_xxx_yy_zec_reth.bin` in the default `reth-inputs` folder, where:

- `cccccc` is the chain name (i.e. mainnet)
- `xxx` is the number of transactions in the block
- `yy` is the gas used in megagas (MGas)

To place the file elsewhere, use the `-o` flag:

```bash
target/release/input-gen -o ./output rpc --block 22767493 -u <RPC_URL>
```

# Zisk Ethereum Client

An experimental Ethereum execution client built for the ZisK zkVM.
It allows you to build, run, and test Ethereum block execution inside the ZisK emulator.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended).
- [cargo-zisk](https://0xpolygonhermez.github.io/zisk/getting_started/installation.html) (ZisK’s Cargo wrapper).
- A working Ethereum RPC endpoint (e.g. Infura, Alchemy, or your own node) for input generation.

## Build the Client ELF

There are two guest client implementations:
- `zec-rsp-client`, based on [RSP](https://github.com/succinctlabs/rsp)
- `zec-zeth-client`, based on [Zeth](https://github.com/boundless-xyz/zeth)

### Build `zec-rsp-client`:
```bash
cd bin/client/rsp
cargo-zisk build --release
```

The compiled ELF will be generated at:
```bash
./target/riscv64ima-zisk-zkvm-elf/release/zec-rsp-client
```

### Build `zec-zeth-client`:
```bash
cd bin/client/zeth
cargo-zisk build --release
```

The compiled ELF will be generated at:
```bash
./target/riscv64ima-zisk-zkvm-elf/release/zec-zeth-client
```

### Execute Ethereum Blocks

Sample input files for Ethereum blocks are provided in the `inputs` folder of each client.

To run a block in the ZisK emulator, use:
```bash
cargo-zisk run --release -i ./inputs/22767493_185_15.bin
```

Or, directly via the `ziskemu` tool:
```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/zisk-rsp-client -i ./inputs/22767493_185_15.bin
```

## Generate Input Block Files

To generate your own input files, you can use the `input-gen` tool.

Example: generate an input file for block `22767493` for the `zec-rsp-client` guest program:
```bash
cargo run --release -- -b 22767493 -g rsp -r <RPC_URL>
```
Replace `<RPC_URL>` with the URL of an Ethereum Mainnet RPC endpoint.
To generate the input file for the `zec-zeth-client`, use `-g zeth`.

The command will create a file named `22767493_xxx_yy_ggg.bin` in the default `inputs` folder, where:
- `xxx` is the number of transactions in the block
- `yy` is the gas used in megagas (MGas)
- `ggg` is the guest program

To place the file elsewhere, use the `-i` flag:
```bash
cargo run --release -- -b 22767493 -g rsp -r <RPC_URL> -i ./bin/client/rsp/inputs
```

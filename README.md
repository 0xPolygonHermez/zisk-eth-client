# Zisk Ethereum Client

## Build the ELF File

To build the `zisk-eth-client` ELF file, run the following commands:

```bash
cd bin/client
cargo-zisk build --release
```

This will generate the ELF file at the following path:  
`./target/riscv64ima-zisk-zkvm-elf/release/zisk-eth-client`

## Execute

Inside the `data` folder, you will find sample input files for Ethereum blocks. To execute a specific block in the Zisk emulator, run one of the following commands:

```bash
cargo-zisk run --release -i ./inputs/22767493_185_14.bin
```

or

```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/zisk-eth-client -i ./inputs/22767493_185_14.bin
```

## Generate Input Block Files

To generate input files for additional Ethereum blocks, you can use the `input-gen` tool. For example, to generate an input file for block `22075730`, run:

```bash
cargo run --release --bin=input-gen -- -b 22075730 -r <RPC_URL>
```

Replace `<RPC_URL>` with the URL of an Ethereum Mainnet RPC node.

The command will create a file named `22075730_xxx_yy.bin` in the `input` folder (by default), where:
- `xxx` is the number of transactions in the block  
- `yy` is the gas used in megagas (MGas)

To specify a different output folder, use the `-i` flag.

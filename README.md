# ZisK Ethereum Client

An experimental Ethereum execution client built for the [ZisK zkVM](https://github.com/0xPolygonHermez/zisk).

This project enables **stateless block validation** to run, verifying Ethereum blocks without maintaining full blockchain state by using execution witnesses. The validation runs inside the ZisK zkVM, allowing blocks to be proven in real-time.

## Project Structure

```
zisk-eth-client/
├── bin/
│   ├── guests/                          # zkVM guest programs
│   │   ├── stateless-validator-reth/    # Reth-based stateless validator
│   │   └── stateless-validator-ethrex/  # Ethrex-based stateless validator
│   ├── host/                            # Benchmark runner for guest programs
│   │                                    # (also hosts the shared input/hints libraries)
│   ├── input-gen/                       # Thin wrapper around `host::input_gen`
│   └── hints-gen/                       # Thin wrapper around `host::hints_gen`
└── crates/
    ├── guest-reth/                      # Core validation library for reth
    ├── guest-ethrex/                    # Core validation library for ethrex
    └── input/                           # RPC fetching + `ExecutionClient` abstraction
```

The `input-gen` and `hints-gen` binaries are thin wrappers over library
modules in `bin/host` (`host::input_gen`, `host::hints_gen`), so the same
logic can also be invoked programmatically from `host` itself.

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [zisk](https://0xpolygonhermez.github.io/zisk/getting_started/installation.html)
- Ethereum RPC endpoint (Infura, Alchemy, or your own node) for input generation

### Build the Guest Program

To build the Reth stateless validator guest program:

```bash
cd bin/guests/stateless-validator-reth
cargo-zisk build --release
```

The ELF binary will be located at:
```
target/riscv64ima-zisk-zkvm-elf/release/zec-reth
```

### Execute the Program in ZisK

Some input files are available in the `bin/guests/stateless-validator-reth/inputs/` folder for testing.

Run the block validation:

```bash
cd bin/guests/stateless-validator-reth
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/zec-reth \
        -i inputs/<input_file>.bin
```
You can also generate your own inputs using the `input-gen` tool.

### Generate an Input File

```bash
cargo run --release --bin input-gen -- rpc -u <RPC_URL>
```

Fetches the latest block from the RPC endpoint and writes a serialized input file (`<chain>_<block>_<txs>_<mgas>_zec_reth.bin`) under `reth-inputs/`. See [input-gen](#input-gen) for all options.

### Generate Prover Hints

```bash
RUSTFLAGS="--cfg zisk_hints" cargo build --release -p hints-gen
./target/release/hints-gen -f reth-inputs/
```

For each `foo.bin`, a sibling `foo.hints` is written. See [hints-gen](#hints-gen) for details.

## Binaries

### host

Run the Ethereum stateless validator guest against `.bin` inputs.

```bash
host [EXECUTION_OPTIONS] stateless-validator -i <INPUT_FOLDER> [OPTIONS]
```

**Top-level execution flags** (apply to any guest program subcommand):

| Option | Description | Default |
|---|---|---|
| `-a, --action <ACTION>` | `execute`, `verify-constraints`, or `prove` | `execute` |
| `-f, --force-rerun` | Re-run even if results already exist | `false` |
| `-o, --output-folder <PATH>` | Folder for benchmark results | — |
| `-p, --proving-key <PATH>` | Path to proving key (default: installed) | — |
| `-l, --emulator` | Use the emulator backend instead of assembly | `false` |
| `--unlock-mapped-memory` | (mutually exclusive with `--emulator`) | `false` |
| `--gpu` | Use GPU acceleration (verify-constraints / prove only) | `false` |
| `-v, --verbose` | Increase log verbosity (`-v` = debug, `-vv` = trace) | — |

**`stateless-validator` options:**

| Option | Description | Default |
|---|---|---|
| `-i, --input-folder <PATH>` | Folder containing `.bin` inputs (required) | — |
| `-c, --client <CLIENT>` | Execution client: `reth`, `ethrex` | `reth` |
| `--include <PATTERN>` | Only process inputs whose name contains the pattern (repeatable) | — |
| `--exclude <PATTERN>` | Skip inputs whose name contains the pattern (repeatable) | — |

```bash
# Execute (default action)
host stateless-validator -i reth-inputs/ -c reth

# Prove
host --action prove stateless-validator -i reth-inputs/ -c reth

# Filter inputs
host stateless-validator -i reth-inputs/ --include eip4844
```

### input-gen

Generates serialized `.bin` inputs from RPC or EEST fixtures (`reth` and `ethrex`). Full options, support matrix, and output-naming details: see [bin/input-gen/README.md](bin/input-gen/README.md).

### hints-gen

Runs the guest **natively** on existing `.bin` inputs to capture per-block hints for the ZisK prover. Requires `RUSTFLAGS="--cfg zisk_hints"`. Full options and examples: see [bin/hints-gen/README.md](bin/hints-gen/README.md).

## Using a Local ZisK Build

The standard `cargo-zisk` installation fetches the latest published version. If you need to test unreleased features or patches, build ZisK locally from source:

```bash
# Clone and build ZisK
git clone https://github.com/0xPolygonHermez/zisk
cd zisk && cargo build --release
```

Then use the local binaries instead of the installed ones:

```bash
# Build guest with local cargo-zisk
/path/to/zisk/target/release/cargo-zisk build --release

# Execute with local ziskemu
/path/to/zisk/target/release/ziskemu -e <elf> -i <input>
```

## Components

| Component | Description |
|-----------|-------------|
| [**stateless-validator-reth**](bin/guests/stateless-validator-reth/) | zkVM guest program that validates Ethereum blocks statelessly via reth |
| [**stateless-validator-ethrex**](bin/guests/stateless-validator-ethrex/) | zkVM guest program that validates Ethereum blocks statelessly via ethrex |
| [**host**](bin/host/) | Benchmark runner for executing/proving guest programs; also hosts shared input-gen / hints-gen libraries |
| [**input-gen**](bin/input-gen/) | Generate inputs from RPC endpoints or EEST test fixtures (reth + ethrex) |
| [**hints-gen**](bin/hints-gen/) | Run guests natively against `.bin` inputs to capture prover hints |
| [**guest-reth**](crates/guest-reth/) | Core reth validation library: crypto, validation logic, input types |
| [**guest-ethrex**](crates/guest-ethrex/) | Core ethrex validation library: crypto, validation logic, input types |
| [**input**](crates/input/) | RPC data fetching and the shared `ExecutionClient` abstraction (reth, ethrex) |

## Supported Chains

- Ethereum Mainnet
- Sepolia
- Holesky
- Hoodi

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

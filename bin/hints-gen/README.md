# ZisK Hints Generator

Runs ZisK Ethereum Client stateless validator guests **natively** (outside the zkVM) on pre-generated input files in order to capture per-block **hints** for the ZisK prover.

Hints are produced once per block input and reused by the prover, so they only need to be regenerated when the guest logic or input changes.

`hints-gen` is a thin binary wrapper around `host::hints_gen` — the orchestration logic lives in the `host` library crate so it can also be invoked programmatically.

## Building

`hints-gen` must be compiled with the `zisk_hints` cfg flag — without it, the binary builds but refuses to run.

```bash
RUSTFLAGS="--cfg zisk_hints" cargo build --release -p hints-gen
```

## Usage

```bash
hints-gen [OPTIONS] [INPUTS]...
```

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `[INPUTS]...` | One or more `.bin` input files | — |
| `-f, --inputs-folder <PATH>` | Directory of `.bin` input files (processed in sorted order) | — |
| `-o, --output-dir <PATH>` | Output directory for `.hints` files | `<client>-hints/` (e.g. `reth-hints/`) |
| `-c, --client <CLIENT>` | Execution client: `reth`, `ethrex` | `reth` |

Either `[INPUTS]...` or `--inputs-folder` must be provided (they are mutually exclusive).

Logging level is controlled via `RUST_LOG` (default `info`).

## Examples

```bash
# Single input
hints-gen path/to/mainnet_22767493_156_12_zec_reth.bin

# Multiple inputs
hints-gen block1.bin block2.bin block3.bin

# Whole folder, hints written to the default `reth-hints/`
hints-gen -f reth-inputs/

# Whole folder, hints written to a custom directory
hints-gen -f reth-inputs/ -o my-hints/

# Ethrex client
hints-gen -c ethrex -f ethrex-inputs/

# Verbose logging
RUST_LOG=debug hints-gen -f reth-inputs/
```

## Input

`.bin` files produced by [`input-gen`](../input-gen/README.md). The client passed via `-c` must match the client the inputs were generated for.

## Output

For each input `foo.bin`, a `foo.hints` file is written to `<client>-hints/` (override with `-o`).

```
reth-inputs/
  mainnet_22767493_156_12_zec_reth.bin
reth-hints/
  mainnet_22767493_156_12_zec_reth.hints   # generated
```

Per-block execution and total times are logged; when more than one block is processed, an average across the run is reported at the end. Blocks that fail are listed at the end and the process exits non-zero, but other blocks are still processed.

# Stateless Validator Input Generator

Generates serialized input files for the ZisK Ethereum Client stateless validator guest programs.

## Prerequisites

The tool clones [zkevm-benchmark-workload](https://github.com/eth-act/zkevm-benchmark-workload) to generate witness data from Ethereum test fixtures.

## Usage

Input generation is a two-step process:

### Step 1: Generate Witness Fixtures

First, run the script to download and generate witness fixtures from Ethereum test cases:

```bash
./scripts/generate-witness.sh [OPTIONS]
```

This creates witness fixtures organized by gas category in `zkevm-fixtures-input/`.

#### Script Options

All options are passed directly to `witness-generator-cli`. Common options include:

- `--include <PATTERN>` - Filter tests by name pattern
- `--tag <FORK>` - Specify EEST release tag

Check [witness-generator-cli](https://github.com/eth-act/zkevm-benchmark-workload/tree/master/crates/witness-generator-cli) for extensive documentation.

#### Environment Variables

- `RAYON_NUM_THREADS` - Number of parallel threads (default: 10)
- `RUST_LOG` - Log level (default: `info`)

### Step 2: Generate Reth Inputs

Then, run the CLI tool to convert the witness fixtures into serialized inputs for the reth-based guest program:

```bash
cargo run --release -- [OPTIONS]
```

## Output

Generated inputs are organized by gas category in `zkevm-fixtures-input/`:

```
zkevm-fixtures-input/
  1M/
    test_foo.bin
    test_bar.bin
  10M/
    ...
  30M/
    ...
  100M/
    ...
  uncategorized/
    ...
```

## Example

```bash
# Step 1: Generate witness fixtures
./scripts/generate-witness.sh

# Step 1 (filtered): Generate only modexp tests
./scripts/generate-witness.sh --filter modexp

# Step 1 (custom parallelism)
RAYON_NUM_THREADS=4 ./scripts/generate-witness.sh

# Step 2: Generate reth inputs
cargo run --release
```
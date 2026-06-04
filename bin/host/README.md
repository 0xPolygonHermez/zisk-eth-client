# ZisK Ethereum Client Host

Benchmark runner for ZisK Ethereum Client guest programs.

## Building

```bash
cargo build --release -p host
```

## Usage

```bash
host [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `-a, --action <ACTION>` | Action to perform: `execute`, `verify-constraints`, `prove` | `execute` |
| `-f, --force-rerun` | Re-run even if results already exist | `false` |
| `-o, --output-folder <PATH>` | Output folder for benchmark results | None |
| `-k, --proving-key <PATH>` | Path to the proving key file | Required for `verify-constraints`/`prove`. Defaults to installed one |
| `-l, --emulator` | Use emulator backend instead of assembly | `false` |
| `--unlock-mapped-memory` | Use the assembly backend with mapped memory unlocked (mutually exclusive with `--emulator`) | `false` |
| `--gpu` | Use GPU acceleration (verify-constraints / prove only) | `false` |
| `-v, --verbose` | Increase log verbosity (`-v` = debug, `-vv` = trace) | — |

### Commands

#### `stateless-validator`

Run stateless validator benchmarks.

```bash
host stateless-validator [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input-folder <PATH>` | Input folder | Required unless `--hints` is given |
| `-c, --client <CLIENT>` | Execution client: `reth`, `ethrex` | `reth` |
| `--include <PATTERN>` | Include only tests matching pattern (repeatable) | None |
| `--exclude <PATTERN>` | Exclude tests matching pattern (repeatable) | None |
| `--hints <PATH>` | Run against a pre-generated `.hints` file/folder (mutually exclusive with `--input-folder`) | None |
| `--gen-hints` | Generate `.hints` files inline before running (requires `RUSTFLAGS="--cfg zisk_hints"`) | `false` |
| `--hints-out <PATH>` | Output directory for `--gen-hints` (defaults next to inputs) | None |

## Examples

```bash
# Execute benchmarks (default action)
host stateless-validator -i /path/to/input/folder

# Execute with emulator backend
host -l stateless-validator -i /path/to/input/folder

# Filter by pattern (only 10M gas tests)
host stateless-validator -i /path/to/input/folder --include gas-value_10M

# Multiple include patterns (1M or 5M gas tests)
host stateless-validator -i /path/to/input/folder --include gas-value_1M --include gas-value_5M

# Exclude blob tests
host stateless-validator -i /path/to/input/folder --exclude blob

# Use ethrex client
host stateless-validator -c ethrex -i /path/to/input/folder

# Verify constraints
host -a verify-constraints stateless-validator -i /path/to/input/folder

# Generate proof
host -a prove stateless-validator -i /path/to/input/folder

# Force rerun all benchmarks with custom output folder
host -f -o my-results stateless-validator -i /path/to/input/folder

# Use custom proving key
host -a prove -p /path/to/proving.key stateless-validator -i /path/to/input/folder
```

## Output

When `-o/--output-folder` is set, one JSON file per input is written flat into that folder, named after the input's basename:

```
<output-folder>/
  metadata.log
  mainnet_22767493_156_12_zec_reth.json
  mainnet_22781920_84_7_zec_reth.json
  ...
```

Each result file contains:

```json
{
  "test_name": "mainnet_22767493_156_12_zec_reth",
  "time": 1.234,
  "metrics": {
    "steps": 1000000,
    "cost": 5000000,
    "tx_count": 42,
    "gas_used": 850000
  }
}
```

`cost`, `tx_count`, and `gas_used` are optional and may be absent depending on the action and client. `metadata.log` captures the run configuration.
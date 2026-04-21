# ZisK Ethereum Client Host

Benchmark runner for ZisK Ethereum Client guest programs.

## Building

```bash
cargo build --release -p host
```

## Usage

```bash
zec-host [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `-a, --action <ACTION>` | Action to perform: `execute`, `verify-constraints`, `prove` | `execute` |
| `-l, --emulator` | Use emulator backend instead of assembly | `false` |
| `-p, --proving-key <PATH>` | Path to the proving key file | Required for `verify-constraints`/`prove`. Defaults to installed one |
| `-o, --output-folder <PATH>` | Output folder for benchmark results | None |
| `-f, --force-rerun` | Force rerun even if results exist | `false` |

### Commands

#### `stateless-validator`

Run stateless validator benchmarks.

```bash
zec-host stateless-validator [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input-folder <PATH>` | Input folder | Required |
| `-c, --client <CLIENT>` | Execution client: `reth`, `ethrex` | `reth` |
| `--include <PATTERN>` | Include only tests matching pattern (repeatable) | None |
| `--exclude <PATTERN>` | Exclude tests matching pattern (repeatable) | | None |

## Examples

```bash
# Execute benchmarks (default action)
zec-host stateless-validator -i /path/to/input/folder

# Execute with emulator backend
zec-host -l stateless-validator -i /path/to/input/folder

# Filter by pattern (only 10M gas tests)
zec-host stateless-validator -i /path/to/input/folder --include gas-value_10M

# Multiple include patterns (1M or 5M gas tests)
zec-host stateless-validator -i /path/to/input/folder --include gas-value_1M --include gas-value_5M

# Exclude blob tests
zec-host stateless-validator -i /path/to/input/folder --exclude blob

# Use ethrex client
zec-host stateless-validator -c ethrex -i /path/to/input/folder

# Verify constraints
zec-host -a verify-constraints stateless-validator -i /path/to/input/folder

# Generate proof
zec-host -a prove stateless-validator -i /path/to/input/folder

# Force rerun all benchmarks with custom output folder
zec-host -f -o my-results stateless-validator -i /path/to/input/folder

# Use custom proving key
zec-host -a prove -p /path/to/proving.key stateless-validator -i /path/to/input/folder
```

## Output

Results are saved as JSON files preserving the input folder structure:

```
<output-folder>/
  stateless-validator/
    1M/
      test_foo.json
      test_bar.json
    10M/
      ...
```

Each result file contains:

```json
{
  "test_name": "test_foo",
  "time": 1.234,
  "metrics": {
    "steps": 1000000,
    "cost": 5000000,
    "tx_count": 42,
    "gas_used": 850000
  }
}
```

A `metadata.log` file is also written with run configuration.
# ZisK Ethereum Client Host

Benchmark runner for ZisK Ethereum Client guest programs.

## Building

```bash
cargo build --release
```

## Usage

```bash
zec-host [OPTIONS] --elf <ELF> <COMMAND>
```

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `-a, --action <ACTION>` | Action to perform: `execute`, `verify-constraints`, `prove` | `execute` |
| `--elf <ELF>` | Path to the compiled ZisK ELF binary | Required |
| `--ziskemu <PATH>` | Path to ziskemu binary | `ziskemu` (from PATH) |
| `-o, --output-folder <PATH>` | Output folder for benchmark results | `zkevm-metrics` |
| `--force-rerun` | Force rerun even if results exist | `false` |

### Commands

#### `stateless-validator`

Run stateless validator benchmarks.

```bash
zec-host --elf <ELF> stateless-validator [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input-folder <PATH>` | Input folder with benchmark fixtures | `zkevm-fixtures-input` |
| `-c, --client <CLIENT>` | Execution client: `reth` | Required |

## Examples

```bash
# Run stateless validator with reth
zec-host --elf target/riscv64ima-zisk-zkvm-elf/release/zec-sv-reth \
    stateless-validator -c reth -i zkevm-fixtures-input

# Use custom ziskemu binary
zec-host --ziskemu /path/to/ziskemu \
    --elf target/riscv64ima-zisk-zkvm-elf/release/zec-sv-reth \
    stateless-validator -c reth

# Force rerun all benchmarks
zec-host --force-rerun \
    --elf target/riscv64ima-zisk-zkvm-elf/release/zec-sv-reth \
    stateless-validator -c reth

# Custom output folder
zec-host -o my-results \
    --elf target/riscv64ima-zisk-zkvm-elf/release/zec-sv-reth \
    stateless-validator -c reth
```

## Output

Results are saved as JSON files preserving the input folder structure:

```
zkevm-metrics/
  stateless-validator-reth/
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
  "action": "Execute",
  "time": 1.23,
  "metrics": {
    "steps": 1000000,
    "cost": 5000000
  }
}
```
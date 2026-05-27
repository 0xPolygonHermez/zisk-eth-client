# ZisK Input Generator

Generates serialized input files for the ZisK Ethereum Client stateless validator guest programs.

`input-gen` is a thin binary wrapper around `host::input_gen` — the orchestration logic lives in the `host` library crate so it can also be invoked programmatically.

## Building

```bash
cargo build --release -p input-gen
```

## Usage

```bash
input-gen [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --client <CLIENT>` | Execution client: `reth`, `ethrex` | `reth` |
| `-o, --output <PATH>` | Output folder | `<client>-inputs` |

### Commands

#### `rpc` — Generate from RPC endpoint

Fetch blocks directly from an Ethereum RPC endpoint.

```bash
input-gen rpc -u <RPC_URL> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-u, --rpc-url <URL>` | RPC endpoint URL (required; auth credentials may also be embedded in the URL) |
| `-H, --rpc-headers <KEY:VALUE>` | Custom HTTP header (repeatable). Only honored by `reth`; `ethrex` warns and ignores |
| `-l, --last-n-blocks <N>` | Last N blocks |
| `-b, --block <N>` | Specific block number |
| `-r, --range-of-blocks <START> <END>` | Block range (inclusive) |
| `-f, --follow` | Continuously follow new blocks |

**Examples:**

```bash
# Single block
input-gen rpc -u <RPC_URL> -b 22767493

# Range of blocks
input-gen rpc -u <RPC_URL> -r 22767490 22767500

# Last 5 blocks
input-gen rpc -u <RPC_URL> -l 5

# Follow new blocks (Ctrl+C to stop)
input-gen rpc -u <RPC_URL> -f

# Authenticated endpoint via custom header
input-gen rpc -u <RPC_URL> -H "Authorization: Bearer <TOKEN>" -b 22767493

# Ethrex client
input-gen -c ethrex rpc -u <RPC_URL> -b 22767493
```

#### Client support matrix

| Client | `rpc` | `eest` |
|---|---|---|
| `reth` | ✅ | ✅ |
| `ethrex` | ✅ | ❌ |

#### `eest` — Generate from EEST fixtures

Generate inputs from [Ethereum Execution Spec Tests](https://github.com/ethereum/execution-spec-tests) fixtures.

```bash
input-gen eest [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-t, --tag <TAG>` | EEST release tag (default: latest) |
| `-p, --eest-fixtures-path <PATH>` | Local fixtures path (mutually exclusive with `--tag`) |
| `-i, --include <PATTERN>` | Filter tests by name (repeatable) |
| `-e, --exclude <PATTERN>` | Exclude tests by name (repeatable) |
| `--threads <N>` | Number of threads for parallel processing |

**Examples:**

```bash
# Generate from default fixtures
input-gen eest

# Use specific release tag
input-gen eest --tag v3.0.0

# Filter by test name pattern
input-gen eest --include modexp
```

## Output

Generated inputs are saved as `.bin` files with the naming convention:

```
<chain>_<block>_<txs>_<mgas>_zec_<client>.bin
```

Example: `mainnet_22767493_156_12_zec_reth.bin`

- **chain**: Network name (`mainnet`, `sepolia`, `holesky`, `hoodi`; `unknown` for unrecognized chain IDs)
- **block**: Block number
- **txs**: Number of transactions
- **mgas**: Gas used in megagas (MGas)
- **client**: Target execution client

# stateless-validator-reth-by-staticlib

A variant of the Reth-based stateless validator that builds both `libziskos_staticlib.a` and the application (`libzec_reth.a`) as static libraries and links them explicitly via `link.x` using `rust-lld`.

## Build

`build.sh` supports four flavours:

| # | Command | Description |
|---|---------|-------------|
| 1 | `./build.sh 1` | Host binary with `--cfg zisk_hints --cfg zisk_hints_debug` |
| 2 | `./build.sh 2` | Host binary (no hints) |
| 3 | `./build.sh 3` | ZisK zkVM ELF — `libzec_reth.a` + `libziskos_staticlib.a` → ELF via `link.x` |
| 4 | `./build.sh 4` | Bare RISC-V (`riscv64im`) — same, with `-C passes=lower-atomic` and JSON target spec |

## Run

**Before running for the first time, wrap the input files:**

```bash
python3 wrap_inputs.py
```

The bundled `inputs/*.bin` files are in the two-record format used by the original `stateless-validator-reth` (which used the Rust `read_input_slice` wrapper). This variant calls `read_input` directly via the C ABI, which expects a single outer-length-prefixed payload:

```
before:  [8B len1][pub bytes][pad][8B len2][wit bytes][pad]
after:   [8B outer_len]  [8B len1][pub bytes][pad][8B len2][wit bytes][pad]
         └─ == file size of the original
```

`wrap_inputs.py` prepends that 8-byte header in-place. It is idempotent — re-running it on already-wrapped files is safe.

Once the inputs are wrapped, build all four flavours and run each ELF against them:

```bash
./execute_blocks.sh
```

## Files

- `build.sh` — build orchestrator for all four flavours
- `execute_blocks.sh` — builds and executes all flavours against bundled inputs
- `link.x` — linker script defining memory layout (ROM at `0x80000000`, RAM at `0xa0020000`) and heap/stack symbols for the bare-metal target
- `riscv64im-unknown-none-elf.json` — custom target spec with `max-atomic-width: 64`, required so `portable-atomic`'s compile-time CAS check passes; atomics are lowered to load/store sequences by `-C passes=lower-atomic`
- `wrap_inputs.py` — prepends an 8-byte outer length prefix to each `inputs/*.bin` file so they are compatible with the `read_input` C ABI (must be run once before `execute_blocks.sh`)
- `inputs/` — bundled mainnet block inputs
- `elf/` — pre-built ZisK zkVM ELF

## Design notes

Unlike the original `stateless-validator-reth` (which links `ziskos` as a Cargo crate dependency), this variant links `libziskos_staticlib.a` in a separate explicit link step. This allows the guest application to be built as a `no_std` static library targeting bare-metal RISC-V without pulling in the full Cargo dependency tree of `ziskos`.

The `crates/guest-reth` library is `no_std`-compatible behind a `std` feature flag:
- `rayon`-based parallel signature recovery is only used when `std` is enabled; bare-metal targets use sequential iteration
- IO uses the `read_input`/`write_output` C ABI (the standard ziskos IO interface) rather than the Rust `read_input_slice`/`commit` wrappers

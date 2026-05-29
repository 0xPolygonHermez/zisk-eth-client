# guest-zilkworm

Host-side Rust glue for zilkworm — a C++ ZKEVM compiled to a ZisK RISC-V ELF.
All zilkworm-specific integration lives here: native FFI, ELF embedding for
the proving pipeline, and the input-building helpers re-exported from the
submodule.

## What's here

```
crates/guest-zilkworm/
├── Cargo.toml          # deps: ziskos, zisk-sdk, z6m_common (submodule path)
├── build.rs            # cmake build of zilkworm's prover/host_lib (FFI)
│                       # + embed of the prebuilt guest ELF
├── cpp/CMakeLists.txt  # CMake glue; all sources live in the zilkworm submodule
└── src/lib.rs          # ELF const, run(), z6m_common re-export, extern "C" decl
```

Zilkworm's C++ sources are tracked via the `third_party/zilkworm` submodule;
this crate owns nothing under `cpp/` beyond build glue.

## How it wires up

| Capability | reth / ethrex | zilkworm |
|---|---|---|
| `from_rpc` (build `.bin` inputs) | ✅ | ✅ via `z6m_common::fetch_block_and_witness` |
| `process_fixture` (EEST) | ✅ reth only | ❌ (no EEST format yet) |
| `run` native (host) | Rust `guest_{reth,ethrex}::run` | C++ `z6m_run` via FFI |
| Hint capture during `run` | ✅ (Rust mirror of guest) | ❌ — pending C++ instrumentation in `silkworm_dev`/`evmone` |

### Native run — single C++ entry, no Rust round-trip

`guest_zilkworm::run()` is a one-liner over zilkworm's `prover/host_lib`
FFI (`z6m_run`). The C++ entry owns the entire ziskos dance:

- reads input via ziskos's `read_input` (`extern "C"` in
  [ziskos/.../zkvm_io.rs](https://github.com/0xPolygonHermez/zisk));
- dispatches on the leading `is_test` byte (matching `guest_zisk/hello.cpp`);
- runs `StateTransition::run_rlp` / `::run`;
- writes `gas_used` via ziskos's `write_output`.

Same code path runs inside the zkVM ELF and natively for hints — when
zilkworm's EVM calls `hint_*` during execution, those flow through the same
ziskos extern `"C"` symbols that `hints-gen` arms (`init_hints_file` +
`set_native_input`).

### Guest ELF embedding

The zilkworm guest is a prebuilt C++ ELF (built by
[`build-elf.sh`](./build-elf.sh)). This
crate's `build.rs` locates and hashes it, then emits
`ZISK_ELF_z6m_guest` / `ZISK_ELF_HASH_z6m_guest` env vars that this crate's
own `load_program!("z6m_guest")` consumes. `bin/host`'s `elfs.rs` re-exports
the resulting [`ELF`] const.

## Build requirements

- **System gcc/g++ ≥ 13** and **cmake ≥ 3.28** for the host FFI (`silkworm_dev`).
- **xPack `riscv-none-elf-gcc` (15.2+)** for the guest ELF (built with cmake + make).

`build.rs` auto-detects the xPack toolchain (checks `ZISK_TOOLCHAIN_PREFIX`,
defaults to `~/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin`):

| State | Behaviour |
|---|---|
| Toolchain present | runs [`build-elf.sh`](./build-elf.sh) on every source change (make-incremental, fast no-op when nothing changed). |
| Toolchain absent, ELF already built | uses the ELF as-is — no rebuild. |
| Toolchain absent, no ELF | panics with an install hint. |

The script auto-initialises the required zilkworm submodules (`evmone`,
`intx`, `eest-fixtures`) on first run, with HTTPS fallback if SSH isn't set
up for GitHub. Output:
`third_party/zilkworm/prover/guest_zisk/build/z6m_guest.elf`.

`ZILKWORM_DIR` overrides the default submodule path
(`third_party/zilkworm`).

## End-to-end run

`cargo build` builds the C++ guest automatically when the toolchain is
present — no separate step needed.

```bash
# 1. Generate an input via input-gen
cargo run --release -p input-gen -- \
    -c zilkworm rpc -u <RPC_URL> -b <BLOCK_NUMBER>

# 2. Execute the ELF against the input
ziskemu -e third_party/zilkworm/prover/guest_zisk/build/z6m_guest.elf \
        -i zilkworm-inputs/<file>.bin
```

# guest-ziskethone

Host-side glue for the **ziskethone** C++ (evmone-based) ZisK Ethereum guest.

Pattern B (externally-built ELF) per `docs/adding-a-client.md`. Like
`guest-zilkworm`, ziskethone exposes a native FFI run path (`zeg_run`), so this
crate provides:
- `ELF` — the prebuilt guest ELF the prover runs. Committed at
  `bin/guests/stateless-validator-ziskethone/elf/zec-ziskethone.elf` and embedded at
  compile time, so a normal build needs
  no C++/RISC-V toolchain.
- `run()` — the C++ EVM executed in-process on the host via FFI, used for fast
  input checking and **hint generation**. Behind the `native-ffi` feature (off by
  default), since compiling+linking it needs clang >= 15 / g++ >= 12 and the
  ziskethone submodule.

## Features
- `native-ffi` — build + link `libzeg_ffi.a` (evmone/blst) and provide the FFI
  `run()`. Off by default; `input-ziskethone` (and its `native-ffi`
  passthrough, surfaced up the stack as `ziskethone-native-ffi`) opts in when
  the native input checker is wanted. Consumers that only embed `ELF`, or only
  need input generation, leave it off — no C++ toolchain. Without it, `run()` is
  a stub that panics.
- `ziskethone-rebuild-guest` — regenerate the committed `elf/zec-ziskethone.elf`
  from the C++ sources (needs the xPack RISC-V toolchain). On-demand only; commit
  the result. Normal builds embed the checked-in ELF and never run this.

## Layout
- `build.rs` — does nothing on a normal build (the ELF is embedded from the
  committed file). Two on-demand paths, each gated by its feature:
  - `regenerate_committed_elf()` (`ziskethone-rebuild-guest`): runs `build-elf.sh` to
    cross-compile `cpp-guest/zisk` and copies the result over the committed
    `elf/zec-ziskethone.elf` (renamed on copy: the CMake target is
    `zisk_eth_guest.elf`).
  - `build_ffi()` (`native-ffi`): builds the `cpp-guest` native static lib
    (`libzeg_ffi.a`) plus its evmone/blst deps and links them so `run()` can
    call `zeg_run`. Preflights the C++ toolchain and reconciles a stale CMake
    cache first.
- `build-elf.sh` — cmake driver for `cpp-guest/zisk` (target `zisk_eth_guest.elf`).
- `src/lib.rs` — `pub const ELF` (always) and `pub fn run()` (feature `native-ffi`).

## Regenerate the committed ELF
```bash
cargo build -p guest-ziskethone --features ziskethone-rebuild-guest
# then commit bin/guests/stateless-validator-ziskethone/elf/zec-ziskethone.elf
```
`ZISKETHONE_DIR` (default `../../../../third_party/ziskethone`) overrides the source
checkout; `ZISK_TOOLCHAIN_PREFIX` points at the RISC-V toolchain's `bin/`.

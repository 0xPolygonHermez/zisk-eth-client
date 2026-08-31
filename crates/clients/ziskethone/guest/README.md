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
  Installs the toolchains it needs before building.
- `install-xpack.sh` — idempotent xPack `riscv-none-elf-gcc` installer, and the one
  place the version and prefix are pinned. Also used by CI.
- `src/lib.rs` — `pub const ELF` (always) and `pub fn run()` (feature `native-ffi`).

## Regenerate the committed ELF

### Prerequisites

- The `third_party/ziskethone` submodule. The root workspace has a Cargo `path`
  dependency into it, so cargo cannot load the workspace until it exists:

  ```bash
  git submodule update --init --recursive
  ```

- **`g++-13`, `g++-12`, or `g++-11`** on the host — GCC 14's bundled libcody does
  not build under a host g++ newer than ~14, so the build looks for an older one
  and stops if it finds none. Usually already present (Ubuntu 24.04's default
  `g++` is 13.3); otherwise `sudo apt install g++-13` on Debian/Ubuntu. Set
  `CXX`/`CC` to pick one explicitly. Not needed on macOS, where the build uses
  Apple clang.
- `curl`, `tar`, `make`, `cmake`. On macOS, the Xcode Command Line Tools and
  `brew install cmake`.

Nothing else. The cross-toolchain is not a prerequisite: `build-elf.sh` installs
the pinned xPack `riscv-none-elf-gcc` 14.3.0-1 (`install-xpack.sh`) and the
patched GCC 14.3.0 that provides `-mzisk-dma`
(ziskethone's `cpp-guest/patches/gcc/build-toolchain.sh`), then fetches evmone
through CMake. Both installers are idempotent, so later builds re-check them in
about a second.

### Rebuild

```bash
cargo build -p guest-ziskethone --features ziskethone-rebuild-guest
# then commit bin/guests/stateless-validator-ziskethone/elf/zec-ziskethone.elf
```

On a cold machine the toolchain install costs ~10-15 minutes, and cargo hides
build-script output unless you pass `-vv` — so for a first build, run the script
directly to watch it work, then let cargo take over:

```bash
./crates/clients/ziskethone/guest/build-elf.sh
```

The ELF is always built with `-mzisk-dma`; `build-elf.sh` verifies the flag
actually lowered by counting DMA markers in the output and fails if it did not.

### Overrides

- `ZISKETHONE_DIR` (default `../../../../third_party/ziskethone`) — the source checkout.
- `ZISK_TOOLCHAIN_PREFIX` — the xPack toolchain's `bin/` dir. Setting it opts out
  of the automatic install: if the path has no `riscv-none-elf-g++`, the build
  reports that instead of installing over your choice.
- `ZISK_DMA_GCC_PREFIX` (default `~/.local/xPacks/zisk-dma-gcc-14.3.0`) — where the
  patched GCC is installed.

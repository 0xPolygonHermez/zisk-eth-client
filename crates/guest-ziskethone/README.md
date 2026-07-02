# guest-ziskethone

Host-side glue for the **ziskethone** C++ (evmone-based) ZisK Ethereum guest.

Pattern B (externally-built ELF) per `docs/adding-a-client.md`. Like
`guest-zilkworm`, ziskethone exposes a native FFI run path (`zeg_run`), so this
crate provides both:
- `ELF` — the prebuilt guest ELF the prover runs.
- `run()` — the C++ EVM executed in-process on the host via FFI, used for fast
  input checking and **hint generation**.

## Layout
- `build.rs` —
  - `embed_guest_elf()`: locates ziskethone (`ZISKETHONE_DIR`, default
    `../../third_party/ziskethone`), builds `cpp-guest/zisk` via `build-elf.sh`
    when the xPack RISC-V toolchain is present (else uses a prebuilt ELF), and
    embeds the ELF via `load_program!("zisk_eth_guest")`.
  - `build_ffi()`: builds the `cpp-guest` native static lib (`libzeg_ffi.a`)
    plus its evmone/blst deps and links them so the host can call `zeg_run`.
- `build-elf.sh` — cmake driver for `cpp-guest/zisk` (target `zisk_eth_guest.elf`).
- `src/lib.rs` — `pub const ELF` and `pub fn run()`.

## Build the ELF manually
```bash
ZISKETHONE_DIR=/home/roger/ziskethone ./build-elf.sh
```

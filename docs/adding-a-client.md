# Adding an execution client

Adding a client means creating its **guest program** (which produces the ELF the
prover runs) and its **host code** (fetches blocks over RPC and wires up the
CLI). The example below adds a client named `geth`; the existing `reth` and
`ethrex` clients are working references with the same file layout.

The guest comes in two patterns — pick the one that matches your client. The
host code (the second half) is identical either way.

## Guest program — Pattern A: Rust guest compiled by `cargo-zisk`

Use this when the guest is written in Rust (like `reth` and `ethrex`).

1. **`crates/guest-geth/`** — the validation library. Start from
   `crates/guest-ethrex/`: rename the crate in `Cargo.toml` and adapt the
   validation logic. Expose a serializable input type (e.g. `GethInput`). Then
   add it to `[workspace.dependencies]` in the root `Cargo.toml` (the `crates/*`
   glob already makes it a workspace member):
   ```toml
   guest-geth = { path = "crates/guest-geth" }
   ```
2. **`bin/guests/stateless-validator-geth/`** — the zkVM binary (its own nested
   workspace). Start from `bin/guests/stateless-validator-ethrex/`, depend on
   `guest-geth`, and set the package name to `zec-geth`.
3. **`bin/host/build.rs`** — register the guest so `cargo-zisk` builds its ELF
   (this is what lets `load_program!("zec-geth")` resolve):
   ```rust
   build_program("../guests/stateless-validator-geth");
   ```
4. **`bin/host/src/elfs.rs`** — expose the ELF:
   ```rust
   pub const ELF_GETH: GuestProgram = load_program!("zec-geth");
   ```

## Guest program — Pattern B: externally-built ELF embedded via a crate

Use this when the ELF is produced by a non-`cargo-zisk` build — e.g. a C++ guest
compiled with cmake. There's no in-repo example yet (`reth` and `ethrex` are both
Pattern A), so the layout below is the convention to follow: a crate whose
`build.rs` shells out to the external build (e.g. a `build-elf.sh`) and embeds
the resulting ELF.

1. **`crates/guest-geth/`** — a crate that builds and embeds the ELF. Its
   `build.rs` runs the external build; `src/lib.rs` exposes:
   ```rust
   pub const ELF: GuestProgram = load_program!("geth_guest");
   pub fn run() { /* FFI into the native build; used for hint generation */ }
   ```
   It usually also re-exports a block-fetch helper for the input client. Add it
   to `[workspace.dependencies]` in the root `Cargo.toml`:
   ```toml
   guest-geth = { path = "crates/guest-geth" }
   ```
   (No `bin/host/build.rs` change — this crate's own `build.rs` produces the ELF.)
2. **`bin/host/Cargo.toml`** — depend on the crate, for the re-export below:
   ```toml
   guest-geth.workspace = true
   ```
3. **`bin/host/src/elfs.rs`** — re-export the ELF:
   ```rust
   pub use guest_geth::ELF as ELF_GETH;
   ```

## Host (fetches block data over RPC and wires up the CLI)

Identical for both patterns. Each client lives in its own `input-<client>` crate
(mirroring the `guest-*` split), depending on `input-core` for the
`ExecutionClient` trait, `RpcConfig`, and `BlockStats`; the `input` crate is a
thin, feature-gated aggregator that re-exports whichever clients are enabled.

1. **`crates/input-geth/`** (new crate) — `Cargo.toml` depends on
   `input-core.workspace = true`, the guest crate (for its input type or fetch
   helper), and geth's own RPC deps:
   ```toml
   [dependencies]
   input-core.workspace = true
   guest-geth.workspace = true
   # ...RPC deps as needed
   ```
   `src/lib.rs` defines a `GethClient` struct and `impl ExecutionClient` for it
   (importing `ExecutionClient`/`RpcConfig`/`BlockStats` from `input_core`),
   with these methods:
   - `name()` → `"geth"` — used in output filenames and the default
     `<client>-inputs` output dir. (The `--client` flag value comes from the
     `Client` enum variant via its `clap::ValueEnum` derive, not from `name()`.)
   - `display_name()` → `"Geth"` — used in logs.
   - `from_rpc(config, block_number)` → fetch the block and witness over RPC,
     build the guest input, and serialize it into a `ZiskStdin`. (Pattern B
     clients usually fetch via the helper re-exported from their guest crate,
     e.g. `guest_geth::fetch_block_and_witness`.)
   - `run()`.
2. Root `Cargo.toml` `[workspace.dependencies]` — add the new crate (the
   `crates/*` glob already makes it a workspace member):
   ```toml
   input-geth = { path = "crates/input-geth" }
   ```
3. **`crates/input/Cargo.toml`** — add it as an optional dependency and a new
   feature, then add that feature to `default`:
   ```toml
   input-geth = { workspace = true, optional = true }

   [features]
   default = [..., "geth"]
   geth = ["dep:input-geth"]
   ```
4. **`crates/input/src/client.rs`** — add a `#[cfg(feature = "geth")] Geth`
   variant to the `Client` enum, then add the matching `create_client` arm:
   ```rust
   #[cfg(feature = "geth")]
   Client::Geth => Box::new(input_geth::GethClient::default()),
   ```
5. **`crates/input/src/lib.rs`** — re-export the client:
   ```rust
   #[cfg(feature = "geth")]
   pub use input_geth::GethClient;
   ```
6. **`bin/host/src/input_gen/client/geth.rs`** — `impl InputGenClient` for the
   client:
   ```rust
   impl InputGenClient for input::GethClient {
       fn supported_providers(&self) -> &'static [ProviderKind] {
           &[ProviderKind::Rpc]
       }
   }
   ```
   Override `process_fixture` only if the client supports EEST fixtures (as `reth`
   does).
7. **`bin/host/src/input_gen/client/mod.rs`** — add `mod geth;`, then add the
   `create_client` arm:
   ```rust
   Client::Geth => Box::new(input::GethClient::default()),
   ```
8. **`bin/host/src/main.rs`** — map the client to its ELF:
   ```rust
   Client::Geth => ELF_GETH,
   ```

Adding the `Client::Geth` variant makes the build fail until both `create_client`
arms and the `main.rs` ELF match are updated; the compiler errors point you to
each spot.

## Generate inputs and hints, then run

```bash
# 1. Build the guest ELF.
#    Pattern A: cd bin/guests/stateless-validator-geth && cargo-zisk build --release
#    Pattern B: run your client's own build (e.g. crates/guest-geth/build-elf.sh)

# 2. Generate input files from an RPC endpoint → geth-inputs/ (default <client>-inputs).
cargo run --release --bin input-gen -- -c geth rpc -u <RPC_URL> -b <block>

# 3. Generate prover hints for those inputs (writes a .hints next to each .bin).
RUSTFLAGS="--cfg zisk_hints" cargo build --release -p hints-gen
./target/release/hints-gen -f geth-inputs/

# 4. Run the stateless validator over the inputs.
cargo build -p host
host stateless-validator -c geth -i geth-inputs/
```

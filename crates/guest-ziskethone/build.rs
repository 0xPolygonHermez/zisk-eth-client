use std::path::PathBuf;

fn main() {
    embed_guest_elf();
    build_ffi();
}

/// Build the ziskethone `cpp-guest` native static lib (`libzeg_ffi.a`) and
/// link it (plus its evmone/blst dependencies) into this crate, so the host
/// can call `zeg_run` and execute the C++ EVM in-process (zilkworm-style).
///
/// The cmake crate drives the same CMakeLists the standalone executable uses;
/// `.build_target("zeg_ffi")` builds only the static-lib target. evmone's
/// transitive static libs all land under `<out>/build/...`; we add a
/// link-search path for each and link them after `zeg_ffi` (a static
/// archive's dependencies must follow it on the link line). `intx` is
/// header-only in this evmone version, so there is no `intx` archive to link.
fn build_ffi() {
    let cpp_guest = ziskethone_dir().join("cpp-guest");

    println!("cargo:rerun-if-changed={}", cpp_guest.join("src").display());
    println!(
        "cargo:rerun-if-changed={}",
        cpp_guest.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cpp_guest.join("CMakeLists.txt").display()
    );

    let dst = cmake::Config::new(&cpp_guest)
        .build_target("zeg_ffi")
        .build();

    // The cmake crate configures+builds in `<dst>/build`. zeg_ffi lands at
    // the build root; evmone's transitive archives land in fixed subdirs
    // (matching the standalone executable's link line). Add a search path
    // for each.
    let build = dst.join("build");
    for sub in [
        "",
        "lib",
        "_deps/evmone-build/lib/evmone_precompiles",
        "_deps/evmone-build/test_state",
        "_deps/evmone-build/deps/src/blst",
    ] {
        let p = if sub.is_empty() {
            build.clone()
        } else {
            build.join(sub)
        };
        println!("cargo:rustc-link-search=native={}", p.display());
    }

    // evmone's precompiles archive and the ZisK C runtime (libziskc, linked
    // by zisk-sdk) both export a strong `ethash_keccak256` (+ `_32`). Both are
    // standard Keccak-256 and compute the same digest, but two strong globals
    // make the host binary fail with `duplicate symbol: ethash_keccak256`.
    // A `rustc-link-arg` here wouldn't help — it only applies to this crate's
    // own artifacts (an rlib), not the downstream host binary. Instead make
    // evmone's copies local symbols in the precompiles archive so libziskc's
    // stay the sole globals; evmone's intra-archive callers still resolve to
    // their (now-local) definition.
    let precompiles_a =
        build.join("_deps/evmone-build/lib/evmone_precompiles/libevmone_precompiles.a");
    if precompiles_a.exists() {
        // GNU binutils `objcopy` (override with the `OBJCOPY` env var, the cc/cmake
        // convention). Not available on non-GNU toolchains (e.g. macOS llvm), where
        // this localize-symbol trick wouldn't apply anyway.
        let objcopy = std::env::var("OBJCOPY").unwrap_or_else(|_| "objcopy".to_string());
        for sym in ["ethash_keccak256", "ethash_keccak256_32"] {
            let st = std::process::Command::new(&objcopy)
                .arg(format!("--localize-symbol={sym}"))
                .arg(&precompiles_a)
                .status();
            match st {
                Ok(s) if s.success() => {}
                Ok(s) => panic!(
                    "`{objcopy} --localize-symbol={sym} {}` exited with {s}",
                    precompiles_a.display()
                ),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => panic!(
                    "`{objcopy}` not found — GNU binutils objcopy is required to localize \
                     evmone's duplicate `ethash_keccak256` symbol. Install binutils (Debian/Ubuntu: \
                     `apt install binutils`), or set the `OBJCOPY` env var to your toolchain's objcopy."
                ),
                Err(e) => panic!(
                    "failed to run `{objcopy} --localize-symbol={sym}` on {}: {e}",
                    precompiles_a.display()
                ),
            }
        }
    }

    // Link order: the FFI archive first, then its dependencies. evmc is
    // bundled into libevmone in evmone v0.21, so there is no separate evmc
    // archive. stdc++ resolves the C++ runtime symbols.
    println!("cargo:rustc-link-lib=static=zeg_ffi");
    println!("cargo:rustc-link-lib=static=evmone");
    println!("cargo:rustc-link-lib=static=evmone-state");
    println!("cargo:rustc-link-lib=static=evmone_precompiles");
    println!("cargo:rustc-link-lib=static=blst");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

/// Locate (and build if possible) the ziskethone guest ELF, then hash it and
/// emit the `ZISK_ELF_zisk_eth_guest` / `ZISK_ELF_HASH_zisk_eth_guest` env vars
/// that this crate's `load_program!("zisk_eth_guest")` consumes.
///
/// Build policy mirrors guest-zilkworm:
/// 1. If the xPack RISC-V toolchain is present, drive the build via
///    `build-elf.sh` (cmake/make is incremental, so re-runs are cheap).
/// 2. If the toolchain isn't present and the ELF already exists, use it as-is.
/// 3. If neither: panic with a clear install hint.
fn embed_guest_elf() {
    let ziskethone_dir = ziskethone_dir();
    let guest_dir = ziskethone_dir.join("cpp-guest").join("zisk");
    let elf_path = guest_dir.join("build").join("zisk_eth_guest.elf");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build-elf.sh");

    println!("cargo:rerun-if-changed={}", guest_dir.display());
    println!(
        "cargo:rerun-if-changed={}",
        ziskethone_dir.join("cpp-guest").join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ziskethone_dir.join("cpp-guest").join("include").display()
    );
    println!("cargo:rerun-if-changed={}", script.display());
    println!("cargo:rerun-if-env-changed=ZISKETHONE_DIR");
    println!("cargo:rerun-if-env-changed=ZISK_TOOLCHAIN_PREFIX");

    if has_riscv_toolchain() {
        let status = std::process::Command::new("bash")
            .arg(&script)
            .arg(format!("--ziskethone={}", ziskethone_dir.display()))
            .status()
            .unwrap_or_else(|e| panic!("failed to invoke {}: {e}", script.display()));
        assert!(status.success(), "ziskethone guest build failed");
    } else if !elf_path.exists() {
        panic!(
            "ziskethone guest ELF not found at {} and no xPack RISC-V toolchain detected.\n\
             Install xPack `riscv-none-elf-gcc` (15.2+) and point ZISK_TOOLCHAIN_PREFIX\n\
             at its `bin/` dir — default is `~/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin`\n\
             — then re-run `cargo build`. Or build the ELF manually:\n    {}",
            elf_path.display(),
            script.display(),
        );
    }

    let bytes = std::fs::read(&elf_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", elf_path.display()));
    let hash = blake3::hash(&bytes).to_hex().to_string();

    println!("cargo:rerun-if-changed={}", elf_path.display());
    println!(
        "cargo:rustc-env=ZISK_ELF_zisk_eth_guest={}",
        elf_path.display()
    );
    println!("cargo:rustc-env=ZISK_ELF_HASH_zisk_eth_guest={hash}");
}

fn ziskethone_dir() -> PathBuf {
    // Default to the `third_party/ziskethone` submodule. `ZISKETHONE_DIR` overrides
    // (e.g. to point at a local working checkout of ziskethone).
    std::env::var("ZISKETHONE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ziskethone")
        })
}

fn has_riscv_toolchain() -> bool {
    let prefix = std::env::var("ZISK_TOOLCHAIN_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin")
        });
    prefix.join("riscv-none-elf-gcc").exists()
}

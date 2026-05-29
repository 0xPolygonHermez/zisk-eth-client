use std::path::PathBuf;

fn main() {
    build_ffi();
    embed_guest_elf();
}

fn build_ffi() {
    let cpp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");

    let mut cfg = cmake::Config::new(&cpp_dir);
    cfg.define("CMAKE_BUILD_TYPE", "Release");
    cfg.define("Z6M_ROOT", zilkworm_dir());

    println!("cargo:rerun-if-env-changed=ZILKWORM_DIR");

    let dst = cfg.build();

    let build = dst.join("build");
    for sub in [
        "",
        "host_lib",
        "zilk_core/dev",
        "zilk_core/core",
        "third_party",
        "deps/src/blst",
    ] {
        println!(
            "cargo:rustc-link-search=native={}",
            build.join(sub).display()
        );
    }

    println!("cargo:rustc-link-lib=static=z6m_host");
    println!("cargo:rustc-link-lib=static=silkworm_dev");
    println!("cargo:rustc-link-lib=static=silkworm_core");
    println!("cargo:rustc-link-lib=static=evmone");
    println!("cargo:rustc-link-lib=static=blst");
    println!("cargo:rustc-link-lib=stdc++");

    println!("cargo:rerun-if-changed={}", cpp_dir.display());
}

/// Locate (and build if possible) the zilkworm guest ELF, then hash it and
/// emit the `ZISK_ELF_z6m_guest` / `ZISK_ELF_HASH_z6m_guest` env vars that
/// this crate's `load_program!("z6m_guest")` consumes.
///
/// Build policy:
///
/// 1. If the xPack RISC-V toolchain is present, drive the build via
///    `build-elf.sh` (next to this build.rs). cmake/make is incremental, so
///    re-runs are cheap no-ops when nothing changed — matches the
///    `cargo-zisk build_program` UX used by the reth/ethrex guests.
/// 2. If the toolchain isn't present and the ELF already exists, use it as-is.
/// 3. If neither: panic with a clear install hint.
fn embed_guest_elf() {
    let zilkworm_dir = zilkworm_dir();
    let guest_dir = zilkworm_dir.join("prover").join("guest_zisk");
    let elf_path = guest_dir.join("build").join("z6m_guest.elf");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build-elf.sh");

    // Watch the inputs that influence the ELF; cargo re-runs build.rs only
    // when one of these changes (or build.rs itself does).
    println!("cargo:rerun-if-changed={}", guest_dir.display());
    println!(
        "cargo:rerun-if-changed={}",
        zilkworm_dir.join("zilk_core").display()
    );
    println!("cargo:rerun-if-changed={}", script.display());
    println!("cargo:rerun-if-env-changed=ZILKWORM_DIR");
    println!("cargo:rerun-if-env-changed=ZISK_TOOLCHAIN_PREFIX");

    if has_riscv_toolchain() {
        let status = std::process::Command::new("bash")
            .arg(&script)
            .arg(format!("--zilkworm={}", zilkworm_dir.display()))
            .status()
            .unwrap_or_else(|e| panic!("failed to invoke {}: {e}", script.display()));
        assert!(status.success(), "zilkworm guest build failed");
    } else if !elf_path.exists() {
        panic!(
            "zilkworm guest ELF not found at {} and no xPack RISC-V toolchain detected.\n\
             Install xPack `riscv-none-elf-gcc` (15.2+) and point ZISK_TOOLCHAIN_PREFIX\n\
             at its `bin/` dir — default is `~/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin`\n\
             — then re-run `cargo build`. Or build the ELF manually:\n    {}",
            elf_path.display(),
            script.display(),
        );
    }
    // else: toolchain absent but ELF on disk — graceful, use as-is.

    let bytes = std::fs::read(&elf_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", elf_path.display()));
    let hash = blake3::hash(&bytes).to_hex().to_string();

    println!("cargo:rerun-if-changed={}", elf_path.display());
    println!("cargo:rustc-env=ZISK_ELF_z6m_guest={}", elf_path.display());
    println!("cargo:rustc-env=ZISK_ELF_HASH_z6m_guest={hash}");
}

fn zilkworm_dir() -> PathBuf {
    std::env::var("ZILKWORM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third_party/zilkworm")
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

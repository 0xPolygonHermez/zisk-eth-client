use std::path::PathBuf;

fn main() {
    // The guest ELF is committed (crates/clients/ziskethone/guest/elf/) and embedded by
    // `load_program!` at compile time — build.rs does NOT touch it on a normal
    // build, so no xPack RISC-V toolchain is ever needed just to use `ELF`.
    // Regenerate it only on demand, when the C++ guest changed:
    if std::env::var_os("CARGO_FEATURE_REBUILD_GUEST").is_some() {
        ensure_submodule_initialized();
        regenerate_committed_elf();
    }

    // `run` (the native `zeg_run` FFI) is always present in the source, but the
    // C++ static lib it links is only *built* under the `native-ffi` feature —
    // building it needs a C++ toolchain (clang >= 15 / g++ >= 12) and the
    // submodule. Enabling `native-ffi` is what makes calling `run` link; consumers
    // that only embed `ELF` (e.g. the host's proving path) leave it off and need
    // no C++ toolchain. `input-ziskethone` turns it on, since it calls `run`.
    if std::env::var_os("CARGO_FEATURE_NATIVE_FFI").is_some() {
        ensure_submodule_initialized();
        build_ffi();
    }
}

/// Self-heal a fresh clone that forgot `git submodule update --init --recursive`.
/// Mirrors pil2-proofman's build.rs. Note this only rescues builds of *this*
/// crate; the root workspace has a `path` dependency into the submodule, which
/// cargo resolves before any build.rs runs — so a fresh root build still needs
/// `setup.sh` (or the manual submodule command). See setup.sh.
fn ensure_submodule_initialized() {
    let dir = ziskethone_dir();
    // A checked-out submodule has a `.git` entry; a non-empty dir also counts
    // (e.g. ZISKETHONE_DIR points at a plain working checkout, not a submodule).
    let initialized = dir.join(".git").exists()
        || std::fs::read_dir(&dir)
            .map(|mut e| e.next().is_some())
            .unwrap_or(false);
    if initialized {
        return;
    }
    eprintln!(
        "ziskethone submodule not initialized at {}; running git submodule update…",
        dir.display()
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let status = std::process::Command::new("git")
        .args(["submodule", "update", "--init", "--recursive"])
        .current_dir(&root)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`git submodule update --init --recursive` failed with {s}"),
        Err(e) => panic!(
            "failed to run git to init the ziskethone submodule ({e}).\n\
             Run `git submodule update --init --recursive` (or ./setup.sh) manually."
        ),
    }
}

/// Resolve the C++ compiler cmake-rs will hand to CMake, following the same
/// env precedence cc-rs uses: `CXX_<target-with-dashes>`, then
/// `CXX_<target_with_underscores>`, then `CXX`, then the default `c++`.
fn resolve_cxx() -> String {
    let target = std::env::var("TARGET").unwrap_or_default();
    std::env::var(format!("CXX_{target}"))
        .or_else(|_| std::env::var(format!("CXX_{}", target.replace('-', "_"))))
        .or_else(|_| std::env::var("CXX"))
        .unwrap_or_else(|_| "c++".to_string())
}

/// Fail fast if the C++ compiler cmake-rs will use is too old to build
/// evmone v0.21's precompiles, instead of letting the user drown in ~200
/// lines of C++ template errors from deep inside evmone/libstdc++.
///
/// evmone v0.21 needs a C++20-complete toolchain. Two independent floors,
/// both seen in the wild on stock Ubuntu 22.04:
///   * Clang < 15 rejects `Curve::Fp` used as a dependent type without
///     `typename` in evmone's `ecc.hpp` (a C++20 relaxation Clang landed in 15).
///   * libstdc++ < 12 can't model `std::ranges::reverse_view<std::span<…>>`
///     as a range, so `modexp.cpp`'s `std::views::reverse(span)` won't compile
///     — this bites even a *new* Clang when it falls back to GCC 11 headers.
///   * GCC itself needs >= 12 for the same libstdc++ reason.
///
/// Best-effort — if we can't identify the compiler we stay quiet and let the
/// real build speak, so this never blocks an otherwise-fine toolchain.
fn preflight_cxx_toolchain(cxx: &str) {
    let Ok(out) = std::process::Command::new(cxx).arg("--version").output() else {
        // Compiler not runnable here (or a cross setup we don't understand);
        // let the real build produce the authoritative error.
        return;
    };
    let banner = String::from_utf8_lossy(&out.stdout).to_lowercase();

    // Parse the first "<major>." we can find; the banner varies by distro
    // ("Ubuntu clang version 14.0.0-…", "g++ (Ubuntu 11.4.0-…) 11.4.0").
    let first_major = |s: &str| -> Option<u32> {
        s.split(|c: char| !c.is_ascii_digit())
            .find(|t| !t.is_empty())
            .and_then(|t| t.parse().ok())
    };

    let remedy = "\nInstall a newer toolchain and point the FFI build at it, e.g.:\n    \
        CC=clang-15 CXX=clang++-15 cargo build   (or any clang >= 15 / g++ >= 12)\n\
        On Debian/Ubuntu also ensure a modern libstdc++ is present: `apt install g++-12`.";

    // `banner` is lowercased above, so match the lowercased form.
    if banner.contains("apple clang") {
        // AppleClang reports its own version numbers (e.g. "Apple clang version
        // 15.0.0"), which don't map to upstream LLVM releases — so the LLVM
        // ">= 15" threshold below doesn't apply. Don't second-guess it; the
        // compile will surface any real incompatibility.
    } else if banner.contains("clang") {
        // Clang: needs >= 15, AND a libstdc++ >= 12 on its include path.
        if let Some(major) = banner
            .split("clang version")
            .nth(1)
            .and_then(first_major)
            .or_else(|| first_major(&banner))
        {
            if major < 15 {
                panic!(
                    "guest-ziskethone FFI build needs Clang >= 15 to compile evmone v0.21, \
                     but `{cxx}` is Clang {major}.{remedy}"
                );
            }
        }
        // A modern clang can still fail on GCC 11's libstdc++ (the modexp
        // ranges bug). Probe the libstdc++ version this clang actually sees.
        if let Some(gv) = probe_libstdcxx_major(cxx) {
            if gv < 12 {
                panic!(
                    "guest-ziskethone FFI build: `{cxx}` resolves to libstdc++ {gv} headers, \
                     but evmone v0.21 needs libstdc++ >= 12 (its `std::views::reverse` over \
                     `std::span` won't compile on 11).{remedy}"
                );
            }
        }
    } else if banner.contains("g++") || banner.contains("gcc") {
        if let Some(major) = first_major(&banner) {
            if major < 12 {
                panic!(
                    "guest-ziskethone FFI build needs g++ >= 12 to compile evmone v0.21, \
                     but `{cxx}` is g++ {major}.{remedy}"
                );
            }
        }
    }
    // Any other compiler (e.g. MSVC): don't second-guess it. (AppleClang is
    // handled explicitly above.)
}

/// CMake silently keeps the compiler it first configured with (recorded in
/// `CMakeCache.txt`) even when a later configure passes a different `CXX` —
/// verified: `CXX=g++` over a clang-14 cache still builds with clang-14. So a
/// user who hits the too-old-toolchain failure, then re-runs with a good
/// `CXX=…`, can sail past the preflight yet still rebuild with the broken
/// compiler. When the pinned compiler differs from the one we'd use now, drop
/// the build tree so cmake-rs reconfigures fresh.
///
/// The compare is a plain string match against `cxx`, not a resolved-path
/// equality — a false mismatch (e.g. `c++` vs the `/usr/bin/g++` it points to)
/// just forces one harmless clean reconfigure, and only ever when someone set
/// `CXX`, which is exactly when they want a fresh build anyway. Scoped to our
/// own `OUT_DIR` (`$OUT_DIR/build`), never a sibling crate.
fn reconcile_stale_cmake_cache(cxx: &str) {
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    let build = PathBuf::from(&out_dir).join("build");
    let Ok(text) = std::fs::read_to_string(build.join("CMakeCache.txt")) else {
        return; // no prior configure — nothing to reconcile
    };

    // Cached line: `CMAKE_CXX_COMPILER:FILEPATH=/usr/bin/clang++-14`.
    let Some(cached) = text.lines().find_map(|l| {
        l.strip_prefix("CMAKE_CXX_COMPILER:")
            .and_then(|rest| rest.split_once('=').map(|(_, v)| v.trim()))
    }) else {
        return;
    };

    // Match on the exact path or the trailing component, so `c++` matches a
    // cached `/usr/bin/c++` without a full PATH/symlink resolution.
    let same = cached == cxx || cached.rsplit('/').next() == Some(cxx.trim_start_matches("./"));
    if same {
        return;
    }

    println!(
        "cargo:warning=guest-ziskethone: CMake cache pins CXX `{cached}` but the build now uses \
         `{cxx}`; wiping {} so cmake reconfigures with the new compiler.",
        build.display()
    );
    // Non-fatal on failure: cmake will still run and surface its own error.
    if let Err(e) = std::fs::remove_dir_all(&build) {
        println!(
            "cargo:warning=guest-ziskethone: could not remove stale {}: {e}",
            build.display()
        );
    }
}

/// Ask the C++ compiler which libstdc++ it would use, via the
/// `_GLIBCXX_RELEASE` macro — which is exactly the libstdc++ major version
/// (11, 12, …), unlike `__GLIBCXX__` (a backport-date stamp that reads the
/// same across majors on distro-patched builds). Returns `None` if it can't
/// be determined (e.g. libc++ instead of libstdc++, or preprocessing fails)
/// — callers treat that as "no opinion".
fn probe_libstdcxx_major(cxx: &str) -> Option<u32> {
    let out = std::process::Command::new(cxx)
        .args(["-x", "c++", "-std=c++20", "-E", "-"])
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(b"#include <version>\n_GLIBCXX_RELEASE\n")
                    .ok();
            }
            child.wait_with_output()
        })
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // The last non-directive token is the expanded macro value (the major),
    // or the literal `_GLIBCXX_RELEASE` if libstdc++ isn't the active library
    // (leaving the macro undefined) — that fails to parse and yields None.
    text.lines()
        .rev()
        .find_map(|l| {
            let t = l.trim();
            (!t.is_empty() && !t.starts_with('#')).then_some(t)
        })?
        .parse()
        .ok()
}

/// The cmake crate drives the same CMakeLists the standalone executable uses;
/// `.build_target("zeg_ffi")` builds only the static-lib target. evmone's
/// transitive static libs all land under `<out>/build/...`; we add a
/// link-search path for each and link them after `zeg_ffi` (a static
/// archive's dependencies must follow it on the link line). `intx` is
/// header-only in this evmone version, so there is no `intx` archive to link.
fn build_ffi() {
    let cxx = resolve_cxx();
    preflight_cxx_toolchain(&cxx);
    reconcile_stale_cmake_cache(&cxx);

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

/// Rebuild the guest ELF from the C++ sources (xPack RISC-V toolchain) and copy
/// it into the committed location `elf/zisk_eth_guest.elf`. Only runs under the
/// `rebuild-guest` feature; the result is meant to be committed. `load_program!`
/// embeds the committed file directly, so a normal build never calls this.
fn regenerate_committed_elf() {
    let ziskethone_dir = ziskethone_dir();
    let cpp_guest = ziskethone_dir.join("cpp-guest");
    let built_elf = cpp_guest
        .join("zisk")
        .join("build")
        .join("zisk_eth_guest.elf");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build-elf.sh");
    let committed_elf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("elf/zisk_eth_guest.elf");

    // Rerun the regeneration when the C++ guest sources or the build driver
    // change, so an edited guest doesn't leave a stale committed ELF behind.
    // Watch the source files and the CMake manifest individually — NOT the whole
    // `zisk/` dir, which contains the CMake `build/` output; watching that would
    // mark the crate perpetually dirty (every rebuild writes into it) and re-run
    // the cross-compile on every subsequent build.
    let guest_src = cpp_guest.join("zisk");
    // Both env vars are read below (ziskethone_dir / has_riscv_toolchain); a
    // change to either must re-run this regeneration.
    println!("cargo:rerun-if-env-changed=ZISKETHONE_DIR");
    println!("cargo:rerun-if-env-changed=ZISK_TOOLCHAIN_PREFIX");
    println!("cargo:rerun-if-changed={}", script.display());
    println!(
        "cargo:rerun-if-changed={}",
        guest_src.join("CMakeLists.txt").display()
    );
    if let Ok(entries) = std::fs::read_dir(&guest_src) {
        for entry in entries.flatten() {
            // Skip the CMake output dir; watch every other source entry.
            if entry.file_name() == "build" {
                continue;
            }
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    if !has_riscv_toolchain() {
        panic!(
            "rebuild-guest requested but no xPack RISC-V toolchain detected.\n\
             Install xPack `riscv-none-elf-gcc` (15.2+) — default location\n\
             `~/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin` (or run ./setup.sh),\n\
             or point ZISK_TOOLCHAIN_PREFIX at its `bin/` dir — then rebuild."
        );
    }

    let status = std::process::Command::new("bash")
        .arg(&script)
        .arg(format!("--ziskethone={}", ziskethone_dir.display()))
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke {}: {e}", script.display()));
    assert!(status.success(), "ziskethone guest build failed");

    if let Some(parent) = committed_elf.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
    }
    std::fs::copy(&built_elf, &committed_elf).unwrap_or_else(|e| {
        panic!(
            "failed to copy {} -> {}: {e}",
            built_elf.display(),
            committed_elf.display()
        )
    });
    println!(
        "cargo:warning=Regenerated committed ELF at {} — commit this file.",
        committed_elf.display()
    );
}

fn ziskethone_dir() -> PathBuf {
    // Default to the `third_party/ziskethone` submodule. `ZISKETHONE_DIR` overrides
    // (e.g. to point at a local working checkout of ziskethone).
    std::env::var("ZISKETHONE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../third_party/ziskethone")
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

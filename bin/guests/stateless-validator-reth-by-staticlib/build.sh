#!/bin/bash
# Orchestrates building libziskos_staticlib.a + libzec_reth.a and linking them
# for RISC-V targets, or building the host binary directly via Cargo.
#
# Usage: build.sh <flavour>
#   1  RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_debug' cargo build --release  (host binary)
#   2  cargo build --release                                                        (host binary)
#   3  cargo +zisk build --release --target riscv64ima-zisk-zkvm-elf              (ZisK zkVM ELF)
#   4  cargo +nightly build --release --target riscv64im-unknown-none-elf          (bare RISC-V ELF)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZISK_DIR="$(cd "$SCRIPT_DIR/../../../../zisk" && pwd)"
LIB_C_DIR="$ZISK_DIR/lib-c/c/lib"

RISCV_BARE="riscv64im-unknown-none-elf"
RISCV_ZISK="riscv64ima-zisk-zkvm-elf"
LINK_X="$SCRIPT_DIR/link.x"

# ── helpers ──────────────────────────────────────────────────────────────────

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
    sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
}

host_triple() {
    rustc -vV | sed -n 's/^host: //p'
}

# Return the path to rust-lld inside the given rustup toolchain.
# $1 = toolchain name (e.g. "zisk", "nightly", "stable")
rust_lld() {
    local toolchain="$1"
    local sysroot host lld
    sysroot=$(rustup run "$toolchain" rustc --print sysroot 2>/dev/null)
    host=$(rustup run "$toolchain" rustc -vV 2>/dev/null | sed -n 's/^host: //p')
    lld="$sysroot/lib/rustlib/$host/bin/rust-lld"
    [[ -x "$lld" ]] || die "rust-lld not found for toolchain '$toolchain' at: $lld"
    echo "$lld"
}

# Build libzec_reth.a (the app as a static library) with --cfg zisk_guest set.
# $1 = extra RUSTFLAGS beyond --cfg zisk_guest (may be empty)
# remaining = full cargo invocation (cargo [+toolchain] build --release --target … )
build_app_staticlib() {
    local extra_flags="$1"; shift
    local flags="--cfg zisk_guest"
    [[ -n "$extra_flags" ]] && flags="$flags $extra_flags"

    echo "==> Building libzec_reth.a  [$*  --lib]"
    (cd "$SCRIPT_DIR" && RUSTFLAGS="$flags" "$@" --lib)
}

# Link libzec_reth.a + libziskos_staticlib.a → ELF using link.x.
# $1 = rustup toolchain name (used to locate rust-lld)
# $2 = target triple (used to locate built .a files and determine output dir)
# $3 = ziskos lib dir (parent of libziskos_staticlib.a)
link_elf() {
    local toolchain="$1"
    local target="$2"
    local ziskos_dir="$3"

    local lld
    lld=$(rust_lld "$toolchain")

    local app_lib="$SCRIPT_DIR/target/$target/release/libzec_reth.a"
    local ziskos_lib="$ziskos_dir/libziskos_staticlib.a"

    # cargo-zisk places binaries under target/elf/…; match that for the ZisK target.
    local out_dir
    if [[ "$target" == "$RISCV_ZISK" ]]; then
        out_dir="$SCRIPT_DIR/target/elf/$target/release"
    else
        out_dir="$SCRIPT_DIR/target/$target/release"
    fi
    mkdir -p "$out_dir"
    local out="$out_dir/zec-reth"

    [[ -f "$app_lib"    ]] || die "app staticlib not found: $app_lib"
    [[ -f "$ziskos_lib" ]] || die "ziskos staticlib not found: $ziskos_lib"

    local ld_args=(
        "$lld" -flavor gnu
        -T "$LINK_X"
        --gc-sections
        --allow-multiple-definition
        "$app_lib"
        "$ziskos_lib"
        -o "$out"
    )

    echo "==> Linking  [${ld_args[*]}]"
    "${ld_args[@]}"
    echo "    → $out"
}

# ── flavours ─────────────────────────────────────────────────────────────────

[[ $# -lt 1 ]] && usage

case "$1" in

1)  # Host binary — RUSTFLAGS with zisk_hints
    HOST=$(host_triple)
    HINTS_FLAGS="--cfg zisk_hints --cfg zisk_hints_debug"

    (cd "$ZISK_DIR" && \
        RUSTFLAGS="$HINTS_FLAGS" \
        cargo build -p ziskos-staticlib --release --target "$HOST")

    ZISKOS_LIB_DIR="$ZISK_DIR/target/$HOST/release"
    echo "==> Building host binary (with zisk_hints)"
    # Use `cargo rustc --bin` so the link flags after `--` apply only to the
    # binary's final link step and not to the lib (staticlib) compilation.
    (cd "$SCRIPT_DIR" && \
        RUSTFLAGS="$HINTS_FLAGS" \
        cargo rustc --bin zec-reth --release -- \
            -L "native=$ZISKOS_LIB_DIR" \
            -l "static=ziskos_staticlib" \
            -L "native=$LIB_C_DIR" \
            -l "static=ziskc" \
            -l pthread -l gmp -l gmpxx -l stdc++ \
            -l gomp -l mpi -l sodium \
            -C "link-arg=-Wl,--allow-multiple-definition")
    ;;

2)  # Host binary — no hints
    HOST=$(host_triple)
    (cd "$ZISK_DIR" && cargo build -p ziskos-staticlib --release --target "$HOST")

    ZISKOS_LIB_DIR="$ZISK_DIR/target/$HOST/release"
    echo "==> Building host binary"
    (cd "$SCRIPT_DIR" && \
        cargo rustc --bin zec-reth --release -- \
            -L "native=$ZISKOS_LIB_DIR" \
            -l "static=ziskos_staticlib" \
            -L "native=$LIB_C_DIR" \
            -l "static=ziskc" \
            -l pthread -l gmp -l gmpxx -l stdc++ \
            -C "link-arg=-Wl,--allow-multiple-definition")
    ;;

3)  # ZisK zkVM staticlib + explicit link
    (cd "$ZISK_DIR" && \
        cargo +zisk build -p ziskos-staticlib --release \
            --target "$RISCV_ZISK" \
            --config 'profile.release.lto="fat"')
    ZISKOS_LIB_DIR="$ZISK_DIR/target/$RISCV_ZISK/release"

    build_app_staticlib "" \
        cargo +zisk build --release \
            --target "$RISCV_ZISK" \
            --config 'profile.release.lto="fat"'

    link_elf "zisk" "$RISCV_ZISK" "$ZISKOS_LIB_DIR"
    ;;

4)  # Bare RISC-V staticlib + explicit link via link.x
    RISCV_SPEC="$SCRIPT_DIR/riscv64im-unknown-none-elf.json"
    # -C passes=lower-atomic: riscv64im has no native atomic instructions; lower
    # them to LR/SC sequences via LLVM's lower-atomic pass.
    # -Z json-target-spec + full JSON path: required so the compiler loads
    # max-atomic-width=64 from the spec, which satisfies portable-atomic's
    # compile-time CAS trait check (the pass then lowers the fake atomics).
    RISCV_FLAGS="-C passes=lower-atomic"

    (cd "$ZISK_DIR" && \
        RUSTFLAGS="$RISCV_FLAGS" \
        cargo +nightly build -p ziskos-staticlib --release \
            --target "$RISCV_SPEC" \
            -Z build-std=core,alloc \
            -Z json-target-spec \
            --config 'profile.release.lto="fat"')
    ZISKOS_LIB_DIR="$ZISK_DIR/target/$RISCV_BARE/release"

    build_app_staticlib "$RISCV_FLAGS" \
        cargo +nightly build --release \
            --target "$RISCV_SPEC" \
            -Z build-std=core,alloc \
            -Z json-target-spec \
            --config 'profile.release.lto="fat"'

    link_elf "nightly" "$RISCV_BARE" "$ZISKOS_LIB_DIR"
    ;;

*)
    usage
    ;;
esac

echo ""
echo "Done (flavour $1)."

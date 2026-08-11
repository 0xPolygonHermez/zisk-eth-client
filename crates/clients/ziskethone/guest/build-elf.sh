#!/usr/bin/env bash
# Build ziskethone's C++ ZisK guest ELF (cpp-guest/zisk).
#
# Usage:
#   ./crates/clients/ziskethone/guest/build-elf.sh [--clean] [--ziskethone=PATH]
#                                           [--toolchain-prefix=PATH]
#
# Output: <ziskethone>/cpp-guest/zisk/build/zisk_eth_guest.elf

set -euo pipefail

# Default matches build.rs: the third_party/ziskethone submodule.
# ZISKETHONE_DIR overrides (e.g. a local working checkout of ziskethone).
ZISKETHONE_DIR="${ZISKETHONE_DIR:-$(cd "$(dirname "$0")/../../../.." && pwd)/third_party/ziskethone}"
TOOLCHAIN_PREFIX="${ZISK_TOOLCHAIN_PREFIX:-$HOME/opt/xpack/xpack-riscv-none-elf-gcc-14.3.0-1/bin}"
CLEAN=0

for arg in "$@"; do
    case "$arg" in
        --clean) CLEAN=1 ;;
        --ziskethone=*) ZISKETHONE_DIR="${arg#*=}" ;;
        --toolchain-prefix=*) TOOLCHAIN_PREFIX="${arg#*=}" ;;
        -h|--help)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            echo "unknown arg: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

GUEST_DIR="$ZISKETHONE_DIR/cpp-guest/zisk"
[ -f "$GUEST_DIR/CMakeLists.txt" ] \
    || { echo "ziskethone cpp-guest/zisk not found at $GUEST_DIR" >&2; exit 1; }
[ -x "$TOOLCHAIN_PREFIX/riscv-none-elf-g++" ] \
    || { echo "riscv-none-elf-g++ not found in $TOOLCHAIN_PREFIX" >&2; exit 1; }

# The guest is always built with -mzisk-dma, which needs the patched GCC 14.3.0
# built from ziskethone's cpp-guest/patches/gcc. build-toolchain.sh is
# idempotent — it detects an installed compiler and returns in about a second —
# so calling it unconditionally is cheap, and a warm prefix and a cold one
# produce the same ELF. That is what makes the CI cache purely a speed
# optimization rather than a correctness input.
#
# Mind the trailing component: ZISK_TOOLCHAIN_PREFIX is a bin/ directory,
# ZISK_XPACK_DIR is its parent. The script reuses that xPack's C++ headers and
# binutils, so its version must match GCC 14.3.0 exactly.
DMA_GCC_PREFIX="${ZISK_DMA_GCC_PREFIX:-$HOME/.local/xPacks/zisk-dma-gcc-14.3.0}"
echo "==> ensuring the patched GCC (-mzisk-dma) is installed"
ZISK_DMA_GCC_PREFIX="$DMA_GCC_PREFIX" \
ZISK_XPACK_DIR="${TOOLCHAIN_PREFIX%/bin}" \
PATH="$TOOLCHAIN_PREFIX:$PATH" \
    "$ZISKETHONE_DIR/cpp-guest/patches/gcc/build-toolchain.sh"

# A compiler configured without a visible target assembler silently loses
# HAVE_AS_RISCV_ATTRIBUTE and miscompiles the guest, while still accepting
# -mzisk-dma. `.attribute arch` in the output is the cheap discriminator.
if ! echo 'int main(){}' | "$DMA_GCC_PREFIX/bin/riscv-none-elf-g++" \
        -x c++ -march=rv64ima_zicsr -mabi=lp64 -S -o - - 2>/dev/null \
        | grep -q '\.attribute[[:space:]]*arch'; then
    echo "ERROR: $DMA_GCC_PREFIX was built without a visible target assembler" >&2
    echo "       (no .attribute arch in its output). Its guest miscompiles." >&2
    echo "       Remove it and re-run with $TOOLCHAIN_PREFIX on PATH." >&2
    exit 1
fi

# The patched compiler goes first: toolchain.cmake picks the first
# riscv-none-elf-g++ on PATH, and the stock one does not know -mzisk-dma.
# It symlinks as/ld/objcopy out of the xPack above, so it is self-sufficient.
export PATH="$DMA_GCC_PREFIX/bin:$TOOLCHAIN_PREFIX:$PATH"

BUILD_DIR="$GUEST_DIR/build"
ELF_PATH="$BUILD_DIR/zisk_eth_guest.elf"

if [ "$CLEAN" = 1 ]; then
    echo "==> removing $BUILD_DIR"
    rm -rf "$BUILD_DIR"
fi

# What the ELF depends on, beyond the sources. CMake refuses to reconfigure when
# CMAKE_CXX_COMPILER moves, so detect the move ourselves and start clean instead
# of handing the user a cache error. Also covers future flag changes.
STAMP_FILE="$BUILD_DIR/.zec-toolchain-stamp"
STAMP_NOW="$(command -v riscv-none-elf-g++) $(riscv-none-elf-g++ -dumpversion) ZEG_ZISK_DMA=ON"
if [ -f "$STAMP_FILE" ] && [ "$(cat "$STAMP_FILE")" != "$STAMP_NOW" ]; then
    echo "==> toolchain or flags changed since last configure; removing $BUILD_DIR"
    rm -rf "$BUILD_DIR"
fi

# Use our own build-stub rather than the developer's cpp-guest/build, and pin the
# guest to it with -DEVMONE_SRC so the tree we patched is the tree we compile.
HOST_DIR="$ZISKETHONE_DIR/cpp-guest"

# FetchContent caches evmone under build-stub/_deps, and the patch loop only
# skips patches already applied to THAT tree. A patch added after the tree was
# fetched is therefore never applied — the build still succeeds and the ELF
# still looks right, it is just missing the patch. That is not theoretical: a
# stale Aug-4 _deps missed 05-evmone-zisk-jumpdest-precompile.patch and the
# resulting guest died in ziskemu with an out-of-range write.
#
# So stamp the patch set itself and start clean whenever it moves.
PATCH_STAMP="$HOST_DIR/build-stub/.zec-patch-stamp"
# Collect the patches explicitly rather than globbing straight into cat: with
# `set -o pipefail`, an unmatched glob makes cat fail, the whole assignment
# fails, and the script exits 1 with no message at all because the 2>/dev/null
# swallowed the only clue. An empty patches/ means something is badly wrong, so
# say so instead of dying silently.
shopt -s nullglob
ZEG_PATCHES=("$HOST_DIR"/patches/*.patch)
shopt -u nullglob
[ ${#ZEG_PATCHES[@]} -gt 0 ] \
    || { echo "no evmone patches found in $HOST_DIR/patches" >&2; exit 1; }
PATCH_NOW="$(cat "${ZEG_PATCHES[@]}" | sha256sum | cut -d' ' -f1)"
if [ -d "$HOST_DIR/build-stub" ] && \
   [ "$(cat "$PATCH_STAMP" 2>/dev/null)" != "$PATCH_NOW" ]; then
    echo "==> evmone patch set changed; removing $HOST_DIR/build-stub"
    rm -rf "$HOST_DIR/build-stub"
fi

echo "==> host cmake configure (fetch evmone, apply patches)"
cmake -S "$HOST_DIR" -B "$HOST_DIR/build-stub"
printf '%s\n' "$PATCH_NOW" > "$PATCH_STAMP"

echo "==> configuring cmake"
cmake \
    -S "$GUEST_DIR" \
    -B "$BUILD_DIR" \
    -DCMAKE_TOOLCHAIN_FILE="$GUEST_DIR/toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release \
    -DEVMONE_SRC="$HOST_DIR/build-stub/_deps/evmone-src" \
    -DZEG_ZISK_DMA=ON \
    -G "Unix Makefiles"

printf '%s\n' "$STAMP_NOW" > "$STAMP_FILE"

echo "==> building zisk_eth_guest.elf"
cmake --build "$BUILD_DIR" --target zisk_eth_guest.elf -j"$(nproc)"

[ -f "$ELF_PATH" ] \
    || { echo "build finished but ELF not at $ELF_PATH" >&2; exit 1; }

# A stock build is not marker-free: the ziskos mem* thunks contain 2 markers of
# their own, so "greater than zero" would pass a non-DMA ELF. Compiler lowering
# emits them inline throughout — thousands of them, against the thunks' 2 — so
# any threshold in between separates the two cleanly. The exact count tracks the
# guest sources and the evmone patch set (7,780 without fused dispatch, 7,784
# with), which is why the check is a threshold and not an expected value.
markers=$(riscv-none-elf-objdump -d "$ELF_PATH" | grep -cE 'csrs[[:space:]]+0x813,' || true)
if [ "$markers" -lt 100 ]; then
    echo "ERROR: only $markers DMA markers in $ELF_PATH; -mzisk-dma did not lower anything." >&2
    echo "       A stock build has 2 (the ziskos thunks); a DMA build has thousands." >&2
    exit 1
fi
echo "==> DMA markers: $markers"

echo
echo "ELF: $ELF_PATH"
echo "Run with: ziskemu -e $ELF_PATH -i <input.bin>"
